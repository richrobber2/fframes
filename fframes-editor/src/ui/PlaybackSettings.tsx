import React from "react";
export function PlaybackSettings({rate, limit, onRate, onLimit}: {rate:number;limit:number;onRate:(n:number)=>void;onLimit:(n:number)=>void}) {
  return <div className="fframes-playback-settings">
    <label>Speed<select aria-label="Playback speed" value={rate} onChange={e=>onRate(Number(e.target.value))}>
      {[0.25,0.5,0.75,1,1.25,1.5,2].map(n=><option key={n} value={n}>{n}×{n===1?' · Normal':''}</option>)}
    </select></label>
    <label>Preview FPS<select aria-label="Preview FPS limit" value={limit} onChange={e=>onLimit(Number(e.target.value))}>
      <option value={0}>Auto</option>{[15,24,30,60].map(n=><option key={n} value={n}>{n} FPS</option>)}
    </select></label>
  </div>;
}
