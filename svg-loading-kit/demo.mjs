import { loadingSvg } from "./loading.mjs";
const get = id => document.getElementById(id);
const options = kind => ({
  kind,
  progress: get("indeterminate").checked ? null : Number(get("progress").value),
  color: get("color").value,
  thickness: Number(get("thickness").value),
  speed: Number(get("speed").value),
  reducedMotion: get("reduce").checked,
  label: kind === "spinner" ? "Loading" : "File upload",
});
const render = () => {
  get("percent").textContent = `${get("progress").value}%`;
  get("progress").disabled = get("indeterminate").checked;
  document.querySelectorAll(".card").forEach(card => {
    card.querySelector(".preview > div").innerHTML = loadingSvg(
      options(card.dataset.kind)
    );
  });
};
document.querySelector(".controls").addEventListener("input", render);
document.querySelectorAll(".card").forEach(card =>
  card.querySelector("button").addEventListener("click", () => {
    const link = document.createElement("a");
    const url = URL.createObjectURL(
      new Blob([loadingSvg(options(card.dataset.kind))], {
        type: "image/svg+xml",
      })
    );
    link.href = url;
    link.download = `loading-${card.dataset.kind}.svg`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    get("status").textContent = `${card.dataset.kind} SVG downloaded.`;
  })
);
render();
