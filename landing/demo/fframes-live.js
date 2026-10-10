// fframes live runtime: runs a fframes video in the page as an interactive animation.
//
// The video is a WASM bridge (`impl_wasm_bridge_for!`). Every animation frame the runtime asks it
// for `render_live(frame)`, a list of DOM operations against the previous frame, and applies them to
// SVG nodes that stay in the page. Viewer input goes back to Rust and shows up in
// `ctx.input()` on the next frame:
//
//   - pointer position in video coordinates and the pressed state,
//   - the `data-action` of the element under the pointer (`input.is_hovered`),
//   - clicks and Enter/Space on `[data-action]` elements (`input.count`, `input.seconds_since`),
//   - form fields anywhere in `valuesRoot` with `data-fframes-value="key"` (`input.value`).
//
// Usage:
//   const meta = await bridge.prepare();
//   const live = mount(bridge, element, { fps: meta.fps, durationInFrames: meta.durationInFrames });
//   live.pause(); live.seek(90); live.play(); live.destroy();

const OP_SET = 0;
const OP_REMOVE = 1;
const OP_TEXT = 2;
const OP_REPLACE = 3;
const SVG_NS = "http://www.w3.org/2000/svg";
const XLINK_NS = "http://www.w3.org/1999/xlink";
const XML_NS = "http://www.w3.org/XML/1998/namespace";
// Keep rendering this long after the last input while paused, so animations started by input finish.
const SETTLE_MS = 3000;

function namespaceOf(name) {
  if (name.startsWith("xlink:")) return XLINK_NS;
  if (name.startsWith("xml:")) return XML_NS;
  return null;
}

function parseElement(markup) {
  const holder = document.createElementNS(SVG_NS, "svg");
  holder.innerHTML = markup;
  return holder.firstElementChild;
}

/**
 * Applies `render_live` operations to `host`, whose only child is the frame's root `<svg>`.
 * `ops` is `[numbers, strings, count]`: `numbers` holds `[op, path length, ...path]` per operation
 * and `strings` their string arguments, each followed by "\0". Returns the root `<svg>`.
 */
export function applyOps(host, ops) {
  const [numbers, packed] = ops;
  if (numbers.length === 0) return host.firstElementChild;
  const strings = packed.split("\0");
  let s = 0;
  // Consecutive operations usually target the same element; reuse it when the path repeats.
  let lastStart = -1;
  let lastLength = -1;
  let node = null;
  for (let i = 0; i < numbers.length; ) {
    const op = numbers[i];
    const length = numbers[i + 1];
    const start = i + 2;
    i = start + length;

    if (op === OP_REPLACE && length === 0) {
      host.innerHTML = strings[s++];
      lastStart = -1;
      continue;
    }

    let same = length === lastLength && node !== null;
    for (let k = 0; same && k < length; k++) same = numbers[start + k] === numbers[lastStart + k];
    if (!same) {
      node = host.firstElementChild;
      for (let k = 0; k < length && node; k++) node = node.children[numbers[start + k]];
      if (!node) throw new Error(`fframes live: no element at path ${Array.from(numbers.subarray(start, i)).join("/")}`);
      lastStart = start;
      lastLength = length;
    }

    if (op === OP_SET) {
      const name = strings[s++];
      const value = strings[s++];
      const ns = namespaceOf(name);
      if (ns) node.setAttributeNS(ns, name, value);
      else node.setAttribute(name, value);
    } else if (op === OP_REMOVE) {
      const name = strings[s++];
      const ns = namespaceOf(name);
      if (ns) node.removeAttributeNS(ns, name.slice(name.indexOf(":") + 1));
      else node.removeAttribute(name);
    } else if (op === OP_TEXT) {
      node.textContent = strings[s++];
    } else if (op === OP_REPLACE) {
      const replacement = parseElement(strings[s++]);
      node.replaceWith(replacement);
      node = replacement;
    }
  }
  return host.firstElementChild;
}

/**
 * Runs the bridge's video inside `element`.
 *
 * Options:
 * - `fps`, `durationInFrames`: from `bridge.prepare()`.
 * - `loop` (default true): wrap the timeline; otherwise stop on the last frame.
 * - `autoplay` (default true): start playing the timeline.
 * - `valuesRoot` (default document): where to look for `[data-fframes-value]` fields.
 * - `strategy` (default "diff"): "replace" rebuilds the whole SVG every frame instead of applying
 *   changes, which is how frames were shown before the live runtime. Only useful to compare.
 * - `onFrame(frame, stats)`: called after each rendered frame. `stats.renderMs` is the time in
 *   WASM (render and diff), `stats.applyMs` the time updating the DOM, `stats.ops` the number of
 *   DOM operations (0 with "replace").
 */
export function mount(bridge, element, options) {
  const {
    fps,
    durationInFrames,
    loop = true,
    autoplay = true,
    valuesRoot = document,
    onFrame = () => {},
  } = options;
  let strategy = options.strategy ?? "diff";
  const stats = { renderMs: 0, applyMs: 0, ops: 0 };
  const duration = Math.max(1, durationInFrames);
  const sessionStart = performance.now();

  let playing = false;
  let frame = 0;
  let clockStart = sessionStart;
  let raf = 0;
  let settleUntil = 0;
  let visible = true;
  let hovered;
  let svg;

  element.style.touchAction = "manipulation";
  bridge.reset_live();

  function render(now) {
    bridge.set_time((now - sessionStart) / 1000);
    const start = performance.now();
    if (strategy === "replace") {
      const markup = bridge.render_frame(BigInt(frame));
      const rendered = performance.now();
      element.innerHTML = markup;
      svg = element.firstElementChild;
      stats.renderMs = rendered - start;
      stats.applyMs = performance.now() - rendered;
      stats.ops = 0;
    } else {
      const ops = bridge.render_live(BigInt(frame));
      const rendered = performance.now();
      svg = applyOps(element, ops);
      stats.renderMs = rendered - start;
      stats.applyMs = performance.now() - rendered;
      stats.ops = ops[2];
    }
    onFrame(frame, stats);
  }

  function tick(now) {
    raf = 0;
    if (!visible || document.hidden) return;
    if (playing) {
      const next = Math.floor(((now - clockStart) / 1000) * fps);
      if (loop) frame = next % duration;
      else if (next >= duration - 1) {
        frame = duration - 1;
        setPlaying(false);
      } else frame = next;
    }
    render(now);
    if (playing || now < settleUntil) raf = requestAnimationFrame(tick);
  }

  function wake() {
    if (!raf && visible && !document.hidden) raf = requestAnimationFrame(tick);
  }

  function poke() {
    settleUntil = performance.now() + SETTLE_MS;
    wake();
  }

  function setPlaying(value) {
    if (value && !playing) clockStart = performance.now() - (frame / fps) * 1000;
    playing = value;
    if (value) wake();
  }

  function toVideo(event) {
    const matrix = svg?.getScreenCTM();
    if (!matrix) return null;
    const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix.inverse());
    return [point.x, point.y];
  }

  const actionOf = target => target?.closest?.("[data-action]")?.getAttribute("data-action") ?? undefined;

  function updateHover(target) {
    const action = actionOf(target);
    if (action !== hovered) {
      hovered = action;
      bridge.set_hovered(action);
    }
  }

  const onPointerMove = event => {
    const point = toVideo(event);
    if (point) bridge.set_pointer(point[0], point[1], true);
    updateHover(event.target);
    poke();
  };
  const onPointerLeave = () => {
    bridge.set_pointer(0, 0, false);
    updateHover(null);
    poke();
  };
  const onPointerDown = event => {
    const point = toVideo(event);
    if (point) bridge.set_pointer_down(true, point[0], point[1]);
    poke();
  };
  const onPointerUp = event => {
    const point = toVideo(event);
    if (point) bridge.set_pointer_down(false, point[0], point[1]);
    // A finger has no hover: once it lifts, nothing is under the pointer.
    if (event.pointerType !== "mouse") onPointerLeave();
    poke();
  };
  const onClick = event => {
    const action = actionOf(event.target);
    if (action) {
      bridge.trigger(action);
      poke();
    }
  };
  const onKeyDown = event => {
    if (event.key !== "Enter" && event.key !== " ") return;
    const action = actionOf(event.target);
    if (!action) return;
    event.preventDefault();
    bridge.trigger(action);
    poke();
  };
  // Keyboard focus counts as hover; focus from a tap or click does not.
  const onFocusIn = event => {
    if (!event.target.matches?.(":focus-visible")) return;
    updateHover(event.target);
    poke();
  };
  const onFocusOut = () => {
    updateHover(null);
    poke();
  };
  const onValue = event => {
    const key = event.target?.getAttribute?.("data-fframes-value");
    if (!key) return;
    bridge.set_value(key, event.target.value);
    poke();
  };
  const onVisibility = () => wake();

  element.addEventListener("pointermove", onPointerMove);
  element.addEventListener("pointerleave", onPointerLeave);
  element.addEventListener("pointerdown", onPointerDown);
  element.addEventListener("pointerup", onPointerUp);
  element.addEventListener("click", onClick);
  element.addEventListener("keydown", onKeyDown);
  element.addEventListener("focusin", onFocusIn);
  element.addEventListener("focusout", onFocusOut);
  valuesRoot.addEventListener("input", onValue);
  document.addEventListener("visibilitychange", onVisibility);

  const observer =
    "IntersectionObserver" in window
      ? new IntersectionObserver(entries => {
          visible = entries[0].isIntersecting;
          wake();
        })
      : null;
  observer?.observe(element);

  for (const field of valuesRoot.querySelectorAll("[data-fframes-value]")) {
    bridge.set_value(field.getAttribute("data-fframes-value"), field.value ?? "");
  }
  render(performance.now());
  setPlaying(autoplay);

  return {
    play: () => setPlaying(true),
    pause: () => setPlaying(false),
    get playing() {
      return playing;
    },
    get frame() {
      return frame;
    },
    get durationInFrames() {
      return duration;
    },
    get fps() {
      return fps;
    },
    get strategy() {
      return strategy;
    },
    /** Switches between "diff" and "replace"; the next frame is sent whole. */
    setStrategy(value) {
      strategy = value;
      bridge.reset_live();
      poke();
    },
    seek(to) {
      frame = Math.max(0, Math.min(duration - 1, Math.round(to)));
      clockStart = performance.now() - (frame / fps) * 1000;
      render(performance.now());
      poke();
    },
    destroy() {
      if (raf) cancelAnimationFrame(raf);
      raf = 0;
      observer?.disconnect();
      element.removeEventListener("pointermove", onPointerMove);
      element.removeEventListener("pointerleave", onPointerLeave);
      element.removeEventListener("pointerdown", onPointerDown);
      element.removeEventListener("pointerup", onPointerUp);
      element.removeEventListener("click", onClick);
      element.removeEventListener("keydown", onKeyDown);
      element.removeEventListener("focusin", onFocusIn);
      element.removeEventListener("focusout", onFocusOut);
      valuesRoot.removeEventListener("input", onValue);
      document.removeEventListener("visibilitychange", onVisibility);
    },
  };
}
