/** Framework-independent SVG loaders. Progress is a percentage, or null for indeterminate. */
export function loadingSvg({
  kind = "bar",
  progress = null,
  color = "#6957e8",
  track = "#e9e7f2",
  thickness = 8,
  speed = 1.4,
  label = "Loading",
  reducedMotion = false,
} = {}) {
  if (!["bar", "ring", "spinner"].includes(kind))
    throw new TypeError("Unknown loader kind");
  const escape = value =>
    String(value).replace(
      /[&<>"']/g,
      c =>
        ({
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          '"': "&quot;",
          "'": "&apos;",
        })[c]
    );
  const value =
    typeof progress === "number" && Number.isFinite(progress)
      ? Math.max(0, Math.min(100, progress))
      : null;
  const stroke = Number.isFinite(thickness)
    ? Math.max(1, Math.min(24, thickness))
    : 8;
  const seconds = Number.isFinite(speed) ? Math.max(0.2, speed) : 1.4;
  const indeterminate = kind === "spinner" || value === null;
  const motion = indeterminate && !reducedMotion;
  const attributes = `xmlns="http://www.w3.org/2000/svg" viewBox="${kind === "bar" ? "0 0 320 32" : "0 0 100 100"}" role="progressbar" aria-label="${escape(label)}" aria-valuemin="0" aria-valuemax="100"${indeterminate ? "" : ` aria-valuenow="${value}"`} style="display:block;width:100%;height:auto;color:${escape(color)}"`;
  // Embedded styles keep downloaded SVGs self-contained and respect the viewer's motion preference.
  const style = `<style>@keyframes loading-kit-spin{to{transform:rotate(360deg)}}@keyframes loading-kit-slide{0%{transform:translateX(-96px)}100%{transform:translateX(320px)}}.loading-kit-motion{animation-duration:${seconds}s;animation-timing-function:linear;animation-iteration-count:infinite;transform-box:view-box;transform-origin:center}@media(prefers-reduced-motion:reduce){.loading-kit-motion{animation:none!important}}</style>`;
  let content;
  if (kind === "bar") {
    const y = (32 - stroke) / 2;
    content = `<rect x="0" y="${y}" width="320" height="${stroke}" rx="${stroke / 2}" fill="${escape(track)}"/><svg x="0" y="0" width="320" height="32" viewBox="0 0 320 32" overflow="hidden"><rect x="${indeterminate ? (motion ? 0 : 112) : 0}" y="${y}" width="${indeterminate ? 96 : (320 * value) / 100}" height="${stroke}" rx="${stroke / 2}" fill="currentColor"${motion ? ' class="loading-kit-motion" style="animation-name:loading-kit-slide"' : ""}/></svg>`;
  } else {
    const radius = 50 - stroke / 2 - 2;
    const circumference = 2 * Math.PI * radius;
    content = `<circle cx="50" cy="50" r="${radius}" fill="none" stroke="${escape(track)}" stroke-width="${stroke}"/><g${motion ? ' class="loading-kit-motion" style="animation-name:loading-kit-spin"' : ""}><circle cx="50" cy="50" r="${radius}" fill="none" stroke="currentColor" stroke-width="${stroke}" stroke-linecap="${value === 0 && !indeterminate ? "butt" : "round"}" stroke-dasharray="${circumference}" stroke-dashoffset="${circumference * (1 - (indeterminate ? 0.25 : value / 100))}" transform="rotate(-90 50 50)"/></g>`;
  }
  return `<svg ${attributes}>${style}${content}</svg>`;
}
