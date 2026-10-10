# SVG loading kit

Copy this folder into another project. The SVG generator has no dependencies; the optional React wrapper requires React. Nothing has been published to a package registry.

Preview from the repository root:

```sh
python3 -m http.server 5190 --bind 127.0.0.1 --directory svg-loading-kit
```

Open http://127.0.0.1:5190. Adjust progress, color, thickness, animation speed, or reduced motion; download any preview as a standalone SVG. Host this directory on any static site service.

## JavaScript / standalone SVG

```js
import { loadingSvg } from "./svg-loading-kit/loading.mjs";
const svg = loadingSvg({ kind: "bar", progress: 65, label: "Uploading file" });
document.querySelector("#upload-progress").innerHTML = svg;
```

`progress` is a percentage clamped to 0–100. Omit it or pass `null` for indeterminate motion. Spinner always represents unknown progress. Invalid/nonfinite percentages become indeterminate. Colors and labels are escaped before inserting into markup.

## React

```tsx
import { LoadingBar, ProgressRing, Spinner } from "./svg-loading-kit/Loading";

<LoadingBar progress={65} color="#6957e8" label="Uploading file" style={{ width: "100%" }} />
<ProgressRing progress={65} thickness={8} style={{ width: 96 }} />
<Spinner speed={1.4} label="Loading results" />
```

`speed` is seconds per cycle. `thickness` is in SVG viewBox units, clamped to 1–24. `track` sets the background color. Use the wrapper's `style` and `className` for sizing/layout. Exported SVGs scale with their viewBox; bars use a 10:1 aspect ratio, rings/spinners are square.

Animations use CSS and respect `prefers-reduced-motion` automatically. Set `reducedMotion` to explicitly disable motion. Determinate loaders expose `aria-valuenow`; unknown progress omits it. Give each instance a descriptive `label`. For decorative loaders hide the wrapper with `aria-hidden` on its parent; announce workflow changes through a separate status region, rather than every progress increment.

SVG animations run in web browsers, including SVGs used as images. Raster exports and video renderers may ignore embedded CSS animation: in fframes, animate the rect width, ring dash offset, and rotation using `frame.seconds()` instead. This kit's download is an SVG, not a video.
