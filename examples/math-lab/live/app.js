// Controls feed the official runtime; Rust/WASM owns every plot and frame.
const $ = id => document.getElementById(id);
const modes = ["fourier", "linear", "chaos"];
const panels = Object.fromEntries(
  modes.map(mode => [mode, $(`${mode}-panel`)])
);
const stages = Object.fromEntries(
  modes.map(mode => [mode, $(`${mode}-stage`)])
);
const players = {};
const bridges = {};
const pending = {};
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
let active = "fourier";
let wasmReady;
let fontsReady;

window.mathLabDebug = { bridges, players };

function field(mode, key) {
  return panels[mode].querySelector(`[data-fframes-value="${key}"]`);
}
function setValue(mode, key, value) {
  const input = field(mode, key);
  input.value = String(value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}
function show(id, value) {
  const target = $(id);
  const next = String(value);
  if (target.textContent !== next) target.textContent = next;
}
function root(mode) {
  return stages[mode].querySelector("svg");
}
function number(mode, key) {
  const value = root(mode)?.getAttribute(`data-${key}`);
  return value === null || value === undefined ? NaN : Number(value);
}
function gapFormat(value) {
  if (!Number.isFinite(value)) return "—";
  return value !== 0 && Math.abs(value) < 0.00001
    ? value.toExponential(2)
    : value.toFixed(5);
}
function format(value, digits = 3) {
  if (!Number.isFinite(value)) return "—";
  return (Math.abs(value) < 0.5 * 10 ** -digits ? 0 : value).toFixed(digits);
}

const recipes = {
  square: {
    note: "Only odd harmonics. Each circle is smaller than the last.",
    insight:
      "Start with <strong>one circle</strong>, then slowly add more. The flat edges sharpen, but little ripples remain near each jump. That persistent overshoot is the <strong>Gibbs phenomenon</strong>.",
  },
  sawtooth: {
    note: "Every harmonic contributes. The signs alternate.",
    insight:
      "The sawtooth uses <strong>every harmonic</strong>. Add circles and watch the sloping edge straighten. Near the jump, the ripples become narrower while their overshoot persists.",
  },
  triangle: {
    note: "Odd harmonics, with amplitudes falling as 1/n².",
    insight:
      "Triangle waves have <strong>no jumps</strong>. Their harmonics shrink much faster, so even a few circles draw a close approximation. Compare five terms here with five in the square wave.",
  },
};
const presets = {
  shear: {
    matrix: [1, 1, 0, 1],
    insight:
      "A <strong>shear</strong> tilts the square into a parallelogram. Its shape changes, but its area stays the same: the determinant remains <strong>1</strong>. Replay it and follow the violet arrow.",
  },
  rotation: {
    matrix: [
      0.7071067811865476, -0.7071067811865476, 0.7071067811865476,
      0.7071067811865476,
    ],
    insight:
      "The final matrix <strong>rotates 45°</strong> and preserves area. The slider blends the matrix entries, so the square shrinks a little on its way to the final rotation. A 45° rotation has <strong>no real eigenvectors</strong>.",
  },
  reflect: {
    matrix: [-1, 0, 0, 1],
    insight:
      "A <strong>reflection</strong> reverses orientation. Along this blend, the square first collapses to a line. Watch the determinant pass through <strong>zero</strong>.",
  },
  singular: {
    matrix: [1, 1, 0.5, 0.5],
    insight:
      "Both columns point along the <strong>same line</strong>, so the plane is flattened onto it. The determinant is <strong>zero</strong>: the transformed square has no area, and many points end up in the same place.",
  },
  identity: {
    matrix: [1, 0, 0, 1],
    insight:
      "The <strong>identity matrix</strong> keeps every point where it started. Every nonzero vector is an eigenvector with eigenvalue <strong>1</strong>. Change a matrix entry to see the grid move.",
  },
};

function recipeFormula(shape, terms) {
  const expansions = {
    square: ["4/π", ["sin(t)", " + ⅓ sin(3t)", " + ⅕ sin(5t)"]],
    sawtooth: ["2/π", ["sin(t)", " − ½ sin(2t)", " + ⅓ sin(3t)"]],
    triangle: ["8/π²", ["sin(t)", " − ¹⁄₉ sin(3t)", " + ¹⁄₂₅ sin(5t)"]],
  };
  const [factor, pieces] = expansions[shape];
  const expression =
    pieces.slice(0, terms).join("") + (terms > 3 ? " + …" : "");
  return `<span>f(t)</span> ≈ ${factor} ${terms === 1 ? expression : `[${expression}]`}`;
}

function fourierLabels() {
  const shape = field("fourier", "wave").value;
  const terms = Number($("terms").value);
  show("terms-value", terms);
  show("speed-value", `${Number($("speed").value)}×`);
  show(
    "harmonic-note",
    shape === "sawtooth"
      ? `${terms} harmonic${terms === 1 ? "" : "s"}, up to n = ${terms}.`
      : `${terms} odd harmonic${terms === 1 ? "" : "s"}, up to n = ${2 * terms - 1}.`
  );
  $("formula").innerHTML = recipeFormula(shape, terms);
  show("formula-note", recipes[shape].note);
  $("insight").innerHTML = recipes[shape].insight;
  $("target-legend").hidden = !$("show-target").checked;
}
function syncFourier() {
  const playing = Boolean(players.fourier?.playing);
  show("play-icon", playing ? "Ⅱ" : "▶︎");
  show("play-label", playing ? "Pause" : "Play");
  $("play").setAttribute(
    "aria-label",
    playing ? "Pause animation" : "Play animation"
  );
  $("play").setAttribute("aria-pressed", String(playing));
}
function syncLinear() {
  show(
    "lin-animate",
    players.linear?.playing
      ? "Ⅱ Pause transformation"
      : "↺ Replay transformation"
  );
  show(
    "lin-motion-note",
    reducedMotion.matches
      ? "Reduced motion is on. Use the slider or replay when ready."
      : "Replay takes 3 seconds."
  );
}
function pauseFourier() {
  players.fourier?.pause();
  syncFourier();
}
function playFourier() {
  const player = players.fourier;
  if (!player) return;
  const phase = Number($("phase").value);
  const speed = Number($("speed").value);
  setValue("fourier", "manual", 0);
  player.seek((phase / (speed * 360)) * 240);
  player.play();
  syncFourier();
}
function pauseLinear() {
  const player = players.linear;
  if (!player) return;
  player.pause();
  const progress = number("linear", "progress");
  if (Number.isFinite(progress)) setValue("linear", "progress", progress);
  setValue("linear", "animate", 0);
  syncLinear();
}
function onFrame(mode, frame) {
  if (mode === "fourier") {
    const phase = number(mode, "phase");
    if (Number.isFinite(phase)) {
      $("phase").value = String(phase);
      show("phase-value", `${Math.round(phase)}°`);
    }
    show("fourier-value", format(number(mode, "value")));
    return;
  }
  if (mode === "linear") {
    const progress = number(mode, "progress");
    const det = number(mode, "det");
    const matrix = (root(mode)?.getAttribute("data-matrix") ?? "")
      .split(",")
      .map(Number);
    const kind = root(mode)?.getAttribute("data-eigen-kind");
    const eigenShown = $("lin-show-eigen").checked;
    if (Number.isFinite(progress)) {
      $("lin-progress").value = String(progress);
      show("lin-progress-value", `${Math.round(progress * 100)}%`);
    }
    show("lin-det", format(det));
    if (Number.isFinite(det))
      show(
        "lin-area",
        Math.abs(det) < 1e-9
          ? "Area × 0 · collapsed to a line or point."
          : `Area × ${format(Math.abs(det))} · orientation ${det < 0 ? "reversed" : "preserved"}.`
      );
    if (matrix.length === 4 && matrix.every(Number.isFinite)) {
      show(
        "lin-effective",
        `[ ${format(matrix[0], 2).padStart(5)}  ${format(matrix[1], 2).padStart(5)} ]\n[ ${format(matrix[2], 2).padStart(5)}  ${format(matrix[3], 2).padStart(5)} ]`
      );
      $("lin-effective").setAttribute(
        "aria-label",
        `Current matrix: row 1, ${format(matrix[0])}, ${format(matrix[1])}; row 2, ${format(matrix[2])}, ${format(matrix[3])}`
      );
    }
    const notes = {
      all: "Every nonzero vector is an eigenvector; there is no special direction to highlight.",
      complex: "No real eigenvectors: no real direction stays on its own line.",
      one: "One eigenvector direction. λ tells you the stretch along that line.",
      two: "Two eigenvector directions. Negative λ reverses the arrow along its line.",
    };
    show(
      "lin-eigen-note",
      eigenShown
        ? (notes[kind] ??
            "Eigenvectors stay on their own line as the matrix acts.")
        : "Eigenvectors stay on their own line as the matrix acts."
    );
    $("lin-eigen-legend").hidden =
      !eigenShown || kind === "all" || kind === "complex";
    if (frame >= 90 && field("linear", "animate").value === "1") {
      players.linear?.pause();
      setValue("linear", "progress", 1);
      setValue("linear", "animate", 0);
      syncLinear();
    }
    return;
  }
  const rate = number(mode, "rate");
  const initial = number(mode, "initial");
  show("chaos-rate-value", format(rate));
  show("chaos-initial-value", format(initial, 4));
  show("chaos-start-a", `A starts at ${format(initial, 4)}`);
  show("chaos-start-b", `B starts at ${format(initial + 0.0001, 4)}`);
  show("chaos-final-gap", gapFormat(number(mode, "final-gap")));
  show("chaos-max-gap", gapFormat(number(mode, "max-gap")));
  show("chaos-max-step", `At step ${number(mode, "max-step")} of 80`);
}

async function loadFonts(bridge) {
  bridge.populate_static_fonts_db_with_static_fonts();
  const faces = [];
  for (let i = 0; ; i++) {
    const font = bridge.get_static_font_data_by_index(i);
    if (!font) break;
    const { info, data } = font;
    if (!info.name) continue;
    const face = new FontFace(info.name, data.slice().buffer, {
      weight: String(info.weight),
      style: String(info.style).toLowerCase(),
    });
    faces.push(face.load().then(loaded => document.fonts.add(loaded)));
  }
  await Promise.all(faces);
}
function runtime() {
  return (wasmReady ??= (async () => {
    globalThis.__fframes_get_video_frame ??= () => null;
    const [wasm, live] = await Promise.all([
      import("./pkg/math_lab.js"),
      import("./fframes-live.js"),
    ]);
    await wasm.default();
    return { create_wasm_bridge: wasm.create_wasm_bridge, mount: live.mount };
  })());
}
function compact(mode) {
  const width = stages[mode].getBoundingClientRect().width;
  if (!width) return;
  const narrow = width < 570;
  const input = field(mode, "compact");
  stages[mode].style.aspectRatio = narrow
    ? { fourier: "500 / 760", linear: "1 / 1", chaos: "500 / 900" }[mode]
    : "5 / 3";
  if (input.value !== String(Number(narrow)))
    setValue(mode, "compact", Number(narrow));
}
async function ensureMounted(mode) {
  if (players[mode]) return players[mode];
  if (pending[mode]) return pending[mode];
  pending[mode] = (async () => {
    const note = panels[mode].querySelector("[data-loading]");
    const controls = panels[mode].querySelectorAll(
      "button, input:not([type=hidden])"
    );
    controls.forEach(control => {
      control.disabled = true;
    });
    panels[mode].setAttribute("aria-busy", "true");
    try {
      const { create_wasm_bridge, mount } = await runtime();
      const bridge = create_wasm_bridge(mode);
      bridges[mode] = bridge;
      await (fontsReady ??= loadFonts(bridge));
      const meta = await bridge.prepare();
      compact(mode);
      players[mode] = mount(bridge, stages[mode], {
        fps: meta.fps,
        durationInFrames: meta.durationInFrames,
        valuesRoot: panels[mode],
        autoplay: false,
        loop: mode === "fourier",
        onFrame: frame => onFrame(mode, frame),
      });
      note.hidden = true;
      panels[mode].dataset.state = "ready";
      controls.forEach(control => {
        control.disabled = false;
      });
      if (
        mode === "fourier" &&
        active === mode &&
        !document.hidden &&
        !reducedMotion.matches
      )
        playFourier();
      syncFourier();
      syncLinear();
      return players[mode];
    } catch (error) {
      console.error(`Math in Motion: ${mode} could not load`, error);
      panels[mode].dataset.state = "error";
      note.dataset.error = "true";
      note.textContent =
        location.protocol === "file:"
          ? "Open this experiment through a local web server. WebAssembly cannot load directly from a file URL."
          : "This live experiment could not load. Reload the page to try again. JavaScript and WebAssembly need to be enabled.";
      return null;
    } finally {
      panels[mode].setAttribute("aria-busy", "false");
    }
  })();
  return pending[mode];
}
function selectTab(mode, focus = false) {
  if (!modes.includes(mode)) return;
  if (active !== mode) {
    pauseFourier();
    pauseLinear();
  }
  active = mode;
  for (const item of modes) {
    const selected = item === mode;
    panels[item].hidden = !selected;
    $(`tab-${item}`).setAttribute("aria-selected", String(selected));
    $(`tab-${item}`).tabIndex = selected ? 0 : -1;
  }
  if (focus) $(`tab-${mode}`).focus();
  compact(mode);
  ensureMounted(mode);
}

for (const mode of modes) {
  const tab = $(`tab-${mode}`);
  tab.addEventListener("click", () => selectTab(mode));
  tab.addEventListener("keydown", event => {
    const index = modes.indexOf(mode);
    const next = {
      ArrowRight: (index + 1) % 3,
      ArrowLeft: (index + 2) % 3,
      Home: 0,
      End: 2,
    }[event.key];
    if (next === undefined) return;
    event.preventDefault();
    selectTab(modes[next], true);
  });
  new ResizeObserver(() => compact(mode)).observe(stages[mode]);
}

document.querySelectorAll("[data-wave]").forEach(button => {
  button.addEventListener("click", () => {
    setValue("fourier", "wave", button.dataset.wave);
    document
      .querySelectorAll("[data-wave]")
      .forEach(item =>
        item.setAttribute("aria-pressed", String(item === button))
      );
    fourierLabels();
  });
});
$("terms").addEventListener("input", fourierLabels);
$("speed").addEventListener("input", () => {
  const playing = Boolean(players.fourier?.playing);
  const phase = number("fourier", "phase");
  pauseFourier();
  if (Number.isFinite(phase)) setValue("fourier", "phase", phase);
  setValue("fourier", "manual", 1);
  fourierLabels();
  // Wait for the original speed input to reach the runtime's valuesRoot listener.
  if (playing) queueMicrotask(playFourier);
});
$("phase").addEventListener("input", () => {
  pauseFourier();
  setValue("fourier", "manual", 1);
  show("phase-value", `${Math.round(Number($("phase").value))}°`);
});
$("show-target").addEventListener("change", event => {
  setValue("fourier", "target", Number(event.target.checked));
  fourierLabels();
});
$("play").addEventListener("click", () =>
  players.fourier?.playing ? pauseFourier() : playFourier()
);
$("reset").addEventListener("click", () => {
  const playing = Boolean(players.fourier?.playing);
  pauseFourier();
  setValue("fourier", "manual", 1);
  setValue("fourier", "phase", 0);
  players.fourier?.seek(0);
  if (playing) playFourier();
});

const matrixFields = ["a", "b", "c", "d"];
for (const key of matrixFields) {
  const input = $(`lin-${key}`);
  input.addEventListener("input", () => {
    pauseLinear();
    const value = input.valueAsNumber;
    if (Number.isFinite(value))
      input.value = String(Math.min(3, Math.max(-3, value)));
    document
      .querySelectorAll("[data-lin-preset]")
      .forEach(button => button.setAttribute("aria-pressed", "false"));
    $("lin-insight").innerHTML =
      "Try setting the two columns to <strong>multiples of each other</strong>. The arrows line up and the determinant reaches zero. Or change one column’s sign to reverse the orientation.";
  });
  input.addEventListener("blur", () => {
    if (!Number.isFinite(input.valueAsNumber))
      setValue("linear", key, key === "a" || key === "d" ? 1 : 0);
  });
}
document.querySelectorAll("[data-lin-preset]").forEach(button => {
  button.addEventListener("click", () => {
    pauseLinear();
    const preset = presets[button.dataset.linPreset];
    matrixFields.forEach((key, i) => setValue("linear", key, preset.matrix[i]));
    setValue("linear", "progress", 1);
    document
      .querySelectorAll("[data-lin-preset]")
      .forEach(item =>
        item.setAttribute("aria-pressed", String(item === button))
      );
    $("lin-insight").innerHTML = preset.insight;
  });
});
$("lin-progress").addEventListener("input", () => {
  // Keep the newly scrubbed value when leaving the animated timeline.
  players.linear?.pause();
  setValue("linear", "animate", 0);
  syncLinear();
});
$("lin-show-eigen").addEventListener("change", event =>
  setValue("linear", "eigen", Number(event.target.checked))
);
$("lin-animate").addEventListener("click", () => {
  const player = players.linear;
  if (!player) return;
  if (player.playing) return pauseLinear();
  setValue("linear", "animate", 1);
  player.seek(0);
  player.play();
  syncLinear();
});

function chaosLabels() {
  const rate = Number($("chaos-rate").value);
  document
    .querySelectorAll("[data-chaos-r]")
    .forEach(button =>
      button.setAttribute(
        "aria-pressed",
        String(Math.abs(Number(button.dataset.chaosR) - rate) < 0.0005)
      )
    );
  const notes = {
    2.8: "At <strong>r = 2.8</strong>, the values settle toward one fixed point. Nearby starts converge toward the same long-run value.",
    3.2: "At <strong>r = 3.2</strong>, the long-run values alternate between two points. Look for two branches above the rate marker.",
    3.83: "At <strong>r = 3.83</strong>, a <strong>three-cycle</strong> appears inside a periodic window. Order can return inside the chaotic region.",
  };
  $("chaos-insight").innerHTML =
    notes[String(rate)] ??
    `With <strong>r = ${rate.toFixed(3)}</strong>, compare the largest gap with the final gap. Nearby trajectories can separate, approach one another, and separate again.`;
}
$("chaos-rate").addEventListener("input", chaosLabels);
document
  .querySelectorAll("[data-chaos-r]")
  .forEach(button =>
    button.addEventListener("click", () =>
      setValue("chaos", "rate", button.dataset.chaosR)
    )
  );
stages.chaos.addEventListener("click", event => {
  const svg = root("chaos");
  const transform = svg?.getScreenCTM();
  if (!transform) return;
  const point = new DOMPoint(event.clientX, event.clientY).matrixTransform(
    transform.inverse()
  );
  const x = number("chaos", "map-x");
  const y = number("chaos", "map-y");
  const width = number("chaos", "map-width");
  const height = number("chaos", "map-height");
  if (
    point.x < x ||
    point.x > x + width ||
    point.y < y ||
    point.y > y + height ||
    !(width > 0)
  )
    return;
  setValue("chaos", "rate", (2.5 + ((point.x - x) / width) * 1.5).toFixed(3));
});

reducedMotion.addEventListener("change", () => {
  if (reducedMotion.matches) {
    pauseFourier();
    pauseLinear();
  }
  syncLinear();
});
document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    pauseFourier();
    pauseLinear();
  }
});
window.addEventListener("pagehide", event => {
  if (!event.persisted)
    Object.values(players).forEach(player => player.destroy());
});
fourierLabels();
chaosLabels();
selectTab("fourier");
