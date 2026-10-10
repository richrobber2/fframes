import {execFileSync} from 'node:child_process';
import {cpSync,mkdirSync,rmSync,existsSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
const dir=path.dirname(fileURLToPath(import.meta.url)),repo=path.resolve(dir,'../..');
const run=(cmd,args,cwd=repo)=>execFileSync(cmd,args,{cwd,stdio:'inherit'});
for(const name of ['hello-world','motion-graphics','beta']){
 const bridge=path.join(repo,'examples',name,'editor/editor-bridge');
 if(!existsSync(path.join(bridge,'pkg/package.json')))run('wasm-pack',['build','--target','web','--dev'],bridge);
}
run('pnpm',['--dir','fframes-editor','build']);
for(const name of ['hello-world','motion-graphics','beta']){
 run('pnpm',['--dir',`examples/${name}/editor`,'exec','vite','build','--target','esnext','--base',`/players/${name}/`]);
 const target=path.join(dir,'public/players',name);rmSync(target,{recursive:true,force:true});cpSync(path.join(repo,'examples',name,'editor/dist'),target,{recursive:true});
}
for(const route of ['','examples/motion-graphics','examples/beta']){const target=path.join(dir,'public',route);mkdirSync(target,{recursive:true});cpSync(path.join(dir,'shell.html'),path.join(target,'index.html'));}
mkdirSync(path.join(dir,'public/__fframes'),{recursive:true});writeFileSync(path.join(dir,'public/__fframes/capabilities'),JSON.stringify({draftExport:false}));
