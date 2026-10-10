import React from "react";
export function MobileScrubber({
  frame,
  duration,
  fps,
  onSeek,
}: {
  frame: number;
  duration: number;
  fps: number;
  onSeek: (frame: number) => void;
}) {
  return (
    <label className="fframes-mobile-scrubber text-xs text-secondary">
      Scrub video
      <input
        type="range"
        aria-label="Video position"
        aria-valuetext={`${(frame / fps).toFixed(2)} seconds`}
        min={0}
        max={Math.max(0, duration - 1)}
        step={1}
        value={frame}
        onChange={event => onSeek(Number(event.target.value))}
      />
    </label>
  );
}
