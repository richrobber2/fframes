// Browser geometry tests. Dimensions come from the DOM, never inferred from CSS.
export const modules = [
 ['navigation','.fframes-navigation'],['media','.fframes-mobile-media, aside[aria-label="Project"]'],
 ['preview','#editor-preview'],['timeline','section[aria-label="Timeline"]'],
 ['scrubber','.fframes-mobile-scrubber'],['playback','[aria-label="Playback controls"]'],
 ['range','.fframes-range-tools'],['speed-fps','.fframes-playback-settings']
];
export function inspectModule(doc,id,selector){
 const nodes=[...doc.querySelectorAll(selector)].filter(n=>n.getClientRects().length&&doc.defaultView.getComputedStyle(n).visibility!=='hidden');
 if(!nodes.length)return {module:id,status:'skipped',reason:'Not displayed in this layout or current UI state'};
 const root=doc.querySelector('#fframes-editor'),bounds=root.getBoundingClientRect(),issues=[];
 for(const node of nodes){const r=node.getBoundingClientRect();if(r.width<=0||r.height<=0)issues.push('Zero-sized module');if(r.left<bounds.left-2||r.right>bounds.right+2)issues.push('Module exceeds editor width');
 for(const child of node.querySelectorAll('button,input,select,label')){if(!child.getClientRects().length)continue;const c=child.getBoundingClientRect();if(c.left<bounds.left-2||c.right>bounds.right+2)issues.push(`${child.getAttribute('aria-label')||child.textContent?.trim().slice(0,40)||child.tagName} exceeds editor width`)}
 if(id==='preview'){const svg=node.querySelector('svg');if(!svg)issues.push('No rendered scene');else{const s=svg.getBoundingClientRect();if(s.width>r.width+2||s.height>r.height+2)issues.push('Scene exceeds preview bounds')}}
 if(id==='timeline'&&node.querySelector('canvas')?.width===0)issues.push('Zero-width timeline canvas');
 }
 return {module:id,status:issues.length?'fail':'pass',issues:[...new Set(issues)]};
}
export function installTests({host,routes,presets,show,getActive,applySize,setTesting,restore}){
 let running=false,cancelled=false,last=null;
 const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
 async function waitReady(path){const deadline=Date.now()+60000;while(getActive()?.path!==path||!getActive()?.ready){if(cancelled)throw Error('Cancelled');if(Date.now()>deadline)throw Error('Example loading timed out');await sleep(100)}}
 async function run(){if(running)throw Error('Tests already running');running=true;cancelled=false;const original=getActive()?.path??'/';const results=[];const start=new Date().toISOString();let interrupted=null;
 try{setTesting(true);for(const path of routes.keys()){if(cancelled)break;show(path,false);await waitReady(path);const doc=getActive().frame.contentDocument;
 for(const [width,height] of presets){if(cancelled)break;applySize(width,height);await sleep(300);host.inert=true;
 const details=doc.querySelector('.fframes-mobile-media');const wasOpen=details?.open;if(details)details.open=true;
 await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
 const checks=modules.map(([id,selector])=>inspectModule(doc,id,selector));
 if(details)details.open=wasOpen;results.push({path,width,height,checks});window.dispatchEvent(new CustomEvent('fframes-test-progress',{detail:{completed:results.length,total:routes.size*presets.length}}));
 }
 }}catch(error){interrupted=error.message}finally{host.inert=false;setTesting(false);restore();show(original,false);running=false}
 last={schemaVersion:1,kind:'browser-module-geometry',startedAt:start,finishedAt:new Date().toISOString(),userAgent:navigator.userAgent,complete:!cancelled&&!interrupted&&results.length===routes.size*presets.length,error:interrupted,results,limitations:['Checks visible geometry and horizontal control reachability; does not certify visual design, audio, interactions or every scene frame.','Hidden modules are skipped, not passed.']};
 window.dispatchEvent(new CustomEvent('fframes-test-complete',{detail:last}));return last;
 }
 function download(){if(!last)return;const url=URL.createObjectURL(new Blob([JSON.stringify(last,null,2)],{type:'application/json'})),a=document.createElement('a');a.href=url;a.download='fframes-module-report.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)}
 return {run,cancel(){cancelled=true},getReport:()=>last,download,modules:modules.map(([id])=>id)};
}
