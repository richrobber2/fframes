export function nextTick(now, previous, limit) {
  if (previous < 0 || limit <= 0) return now;
  const interval = 1 / limit;
  const elapsed = now - previous;
  if (elapsed + 0.00001 < interval) return -1;
  return now - Math.max(0, elapsed - Math.floor((elapsed + 0.00001) / interval) * interval);
}
export function audioTiming(currentFrame, track, fps, rate) {
  const position = currentFrame / fps;
  const start = track.start / fps;
  const end = track.end / fps;
  return {delay: Math.max(0, start - position) / rate, offset: Math.max(0, position - start), duration: Math.max(0, end - Math.max(position, start))};
}

export function setSourceRate(source, rate) { source.playbackRate.value = rate; }
