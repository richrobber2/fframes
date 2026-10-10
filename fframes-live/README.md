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
remove an attribute, replace an element's text, or replace an element whose children changed. They
cross into JS as one `Uint32Array` and one string, however many there are. The SVG nodes stay in the
page, so they keep event handlers, focus and hover state, and the browser does not re-parse and
rebuild the whole frame.

The comparison keeps the previous frame's text and byte ranges into it: unchanged attributes cost a
byte comparison, identical subtrees one comparison of their markup. `svgr!` output is read by a small
dedicated scanner, with `roxmltree` as the fallback for anything else.

Rendering pauses while the element is off screen or the tab is hidden. While the timeline is
paused, input still re-renders for a few seconds so animations started by a click finish.

Options of `mount` besides `fps` and `durationInFrames`: `loop` (default true), `autoplay`
(default true), `valuesRoot` (default `document`), `onFrame(frame, stats)` with `stats.renderMs`
(WASM: render and diff), `stats.applyMs` (DOM update) and `stats.ops`, and `strategy`: `"replace"`
puts the whole SVG into the page every frame instead, only to compare (`live.setStrategy(...)`
switches at runtime).

`keepAlive: true` keeps rendering while the timeline is paused, for frames that move with the
session clock (`ctx.input().time`). `live.trigger("action")` triggers an action from a control
outside the frame, as if a `[data-action]` element was clicked (`landing/math/` uses it for its
phone layout).

## Performance

`examples/dot-field` (2,304 circles, every one moving each frame) is the benchmark; open
`landing/dots/` and switch modes. Headless Chromium with software rendering, pointer moving over the
field, median of 5 runs of 3 seconds:

| | Live diff | Full re-render |
| --- | --- | --- |
| Desktop (1440×900): frames per second | 56 | 30 |
| Desktop: WASM per frame (render + diff) | 3.6 ms | 2.4 ms |
| Desktop: DOM update per frame | 3.1 ms | 11.7 ms |
| Phone (Pixel 7 viewport): frames per second | 55 | 35 |
| Desktop, CPU throttled 4×: frames per second | 10 | 7 |
| Phone, CPU throttled 4×: frames per second | 9 | 6 |

Painting 2,304 circles in software is the same in both modes and dominates under throttling.

To keep a live animation cheap:

- Keep the tree's shape stable. An element whose children change count or tags is replaced whole,
  with everything inside it. Hide elements (`opacity="0"`) instead of leaving them out.
- Round values to what the eye can see (`dot-field` uses quarter pixels and color steps of 4). An
  attribute whose text did not change costs nothing.
- Put animated values on as few attributes as possible; every changed attribute is one DOM call.

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
