import React, { useEffect, useState } from "react";

type Props = {
  fps: number;
  start: number;
  end: number;
  duration: number;
  current: number;
  enabled: boolean;
  onStart: (frame: number) => void;
  onEnd: (frame: number) => void;
  onToggle: () => void;
  onPause: () => void;
};
export function RangeTools({
  fps,
  start,
  end,
  duration,
  current,
  enabled,
  onStart,
  onEnd,
  onToggle,
  onPause,
}: Props) {
  const [startInput, setStartInput] = useState(
    String(Number((start / fps).toFixed(3)))
  );
  const [endInput, setEndInput] = useState(
    String(Number((end / fps).toFixed(3)))
  );
  useEffect(
    () => setStartInput(String(Number((start / fps).toFixed(3)))),
    [start, fps]
  );
  useEffect(
    () => setEndInput(String(Number((end / fps).toFixed(3)))),
    [end, fps]
  );
  const [available, setAvailable] = useState(false);
  const [checked, setChecked] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  useEffect(() => {
    const abort = new AbortController();
    fetch("/__fframes/capabilities", { signal: abort.signal })
      .then(r => (r.ok ? r.json() : null))
      .then(value => setAvailable(value?.draftExport === true))
      .catch(() => {})
      .finally(() => {
        if (!abort.signal.aborted) setChecked(true);
      });
    return () => abort.abort();
  }, []);
  const button =
    "rounded-full px-3 py-1.5 text-xs hover:bg-primary-ghost-hover focus-visible:ring-2 disabled:opacity-50";
  const input =
    "w-20 rounded border border-[var(--color-border)] bg-surface px-2 py-1 text-xs tabular";
  const update = (value: string, setter: (frame: number) => void) => {
    const number = Number(value);
    if (value !== "" && Number.isFinite(number))
      setter(Math.round(number * fps));
  };
  const selectedSeconds = (end - start) / fps;
  const resetRange = () => {
    onStart(0);
    onEnd(duration);
    setMessage("");
  };
  const exportDraft = async () => {
    if (busy) return;
    onPause();
    setBusy(true);
    setMessage("Rendering draft…");
    try {
      const response = await fetch("/__fframes/draft", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          startSeconds: start / fps,
          endSeconds: end / fps,
        }),
      });
      if (!response.ok) throw new Error(await response.text());
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `draft-${start}-${end}.mp4`;
      document.body.append(link);
      link.click();
      link.remove();
      setTimeout(() => URL.revokeObjectURL(url), 60000);
      setMessage("Draft downloaded.");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Export failed.");
    } finally {
      setBusy(false);
    }
  };
  return (
    <section
      aria-label="Selected range"
      className="absolute bottom-20 left-1/2 flex w-max max-w-[95vw] -translate-x-1/2 flex-wrap items-center justify-center gap-2 rounded-xl bg-surface-elevated p-2 text-default shadow-[var(--shadow-300)]"
    >
      <span
        className="basis-full text-center text-xs tabular"
        aria-label="Selected duration"
      >
        {selectedSeconds.toFixed(2)}s selected · {end - start} frames
      </span>
      <label className="text-xs">
        Start (s){" "}
        <input
          aria-label="Range start seconds"
          disabled={busy}
          type="number"
          min="0"
          max={(end - 1) / fps}
          step={1 / fps}
          className={input}
          value={startInput}
          onChange={e => {
            setStartInput(e.target.value);
            update(e.target.value, onStart);
          }}
          onBlur={() => setStartInput(String(Number((start / fps).toFixed(3))))}
        />
      </label>
      <button
        className={button}
        disabled={busy}
        onClick={() => onStart(current)}
      >
        Set start here
      </button>
      <label className="text-xs">
        End (s){" "}
        <input
          aria-label="Range end seconds"
          disabled={busy}
          type="number"
          min={(start + 1) / fps}
          max={duration / fps}
          step={1 / fps}
          className={input}
          value={endInput}
          onChange={e => {
            setEndInput(e.target.value);
            update(e.target.value, onEnd);
          }}
          onBlur={() => setEndInput(String(Number((end / fps).toFixed(3))))}
        />
      </label>
      <button
        className={button}
        disabled={busy}
        onClick={() => onEnd(current + 1)}
      >
        Set end here
      </button>
      <button className={button} aria-pressed={enabled} onClick={onToggle}>
        {enabled ? "Loop range on" : "Loop range off"}
      </button>
      <button
        className={button}
        disabled={busy || (start === 0 && end === duration)}
        onClick={resetRange}
      >
        Reset range
      </button>
      {checked && !available && (
        <span className="basis-full text-center text-xs">
          MP4 export is available in the local preview launcher.
        </span>
      )}
      {available && (
        <button
          className={button}
          disabled={busy || selectedSeconds > 30}
          title="Half-resolution MP4, up to 30 seconds"
          onClick={exportDraft}
        >
          {busy ? "Rendering…" : "Export draft MP4"}
        </button>
      )}
      {selectedSeconds > 30 && available && (
        <span className="text-xs">Select up to 30s to export.</span>
      )}
      {message && (
        <span role="status" className="basis-full text-center text-xs">
          {message}
        </span>
      )}
    </section>
  );
}
