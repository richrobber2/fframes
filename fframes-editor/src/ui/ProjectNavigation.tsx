import React, { useEffect } from "react";
type Scene = {name: string; start: number; end: number};
export function ProjectNavigation({scenes, frame, duration, fps, onScene}: {
  scenes: Scene[] | null; frame: number; duration: number; fps: number; onScene: (frame:number)=>void;
}) {
  useEffect(() => {
    const pause = (event: MessageEvent) => {
      if (event.origin === location.origin && event.source === window.parent && event.data?.type === "fframes-pause") {
        window.dispatchEvent(new CustomEvent("fframes-pause"));
      }
    };
    window.addEventListener("message", pause);
    return () => window.removeEventListener("message", pause);
  }, []);
  const items = scenes?.length ? scenes : [{name:"Full video",start:0,end:duration}];
  const active = Math.max(0, items.findIndex(scene=>frame>=scene.start && frame<scene.end));
  const examples = [{path:"/",name:"Hello World"},{path:"/examples/motion-graphics/",name:"Motion Graphics"},{path:"/examples/beta/",name:"Beta Showcase"}];
  const current = examples.find(example=>example.path!=="/" && location.pathname.replace("/players/", "/examples/").startsWith(example.path))?.path ?? "/";
  const time = (frames:number)=>`${Math.floor(frames/fps/60)}:${String(Math.floor(frames/fps)%60).padStart(2,"0")}`;
  const title = (name:string)=>name.split("::").pop()?.replace(/([a-z])([A-Z])/g,"$1 $2") ?? name;
  return <nav className="fframes-navigation" aria-label="Examples and scenes">
    <label>Example<select aria-label="Choose example" value={current} onChange={event=>{window.parent !== window ? window.parent.postMessage({type:"fframes-switch", path:event.target.value}, location.origin) : location.assign(event.target.value)}}>
      {examples.map(example=><option key={example.path} value={example.path}>{example.name}</option>)}
    </select></label>
    <label>Scene<select aria-label="Choose scene" value={active} onChange={event=>onScene(items[Number(event.target.value)].start)}>
      {items.map((scene,index)=><option key={`${index}-${scene.start}`} value={index}>{index+1}. {title(scene.name)} · {time(scene.start)}</option>)}
    </select></label>
  </nav>;
}
