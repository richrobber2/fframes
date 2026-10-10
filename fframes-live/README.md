# fframes live runtime

`fframes-live.js` runs a fframes `Video` in a web page as an interactive animation. Rendering a
video file is one output of the same code; this is another.

```js
import initWasm, { create_wasm_bridge } from "./pkg/my_video_bridge.js";
import { mount } from "./fframes-live.js";

await initWasm();
const bridge = create_wasm_bridge();
const meta = await bridge.prepare();
const live = mount(bridge, document.querySelector("#stage"), {
  fps: meta.fps,
  durationInFrames: meta.durationInFrames,
});
// live.play(), live.pause(), live.seek(frame), live.frame, live.playing, live.destroy()
```

The bridge is the same `impl_wasm_bridge_for!` crate the editor uses (see `AGENTS.md`, "Editor
preview"). Load the fonts the video uses before mounting (see `landing/index.html`), and map the
bridge's `@fframes/editor` import to a module exporting `load_audio_wasm_callback` with an import
map when the page does not include the editor.

## How it renders

Every animation frame the runtime calls `bridge.render_live(frame)`. Rust renders the frame, compares
it with the previous one (`fframes::live::LiveDiff`) and returns only the DOM operations: set or
remove an attribute, replace an element's text, or replace an element whose children changed. The
SVG nodes stay in the page, so they keep event handlers, focus and hover state, and the browser
does not re-parse and rebuild the whole frame.

Rendering pauses while the element is off screen or the tab is hidden. While the timeline is
paused, input still re-renders for a few seconds so animations started by a click finish.

## Input

What the viewer does reaches Rust through `ctx.input()` (`fframes::Input`):

| In the page | In Rust |
| --- | --- |
| pointer over the video | `input.pointer` (video coordinates), `input.pointer_down` |
| element with `data-action="open"` under the pointer or focused with the keyboard | `input.is_hovered("open")` |
| click, tap, Enter or Space on it | `input.count("open")`, `input.seconds_since("open")` |
| `<input data-fframes-value="name">` inside `valuesRoot` | `input.value("name")` |
| time since mount | `input.time` |

Play an animation from an event with `frame.animate_at(&anim, input.seconds_since("open").unwrap_or(0.))`.
For state of your own, implement `Video::on_event`. File renders pass an untouched input, so the
same video renders its resting state into a file.

Give interactive elements `role="button"`, `tabindex="0"` and an `aria-label` in the `svgr!` markup
so they work from the keyboard and with screen readers.
