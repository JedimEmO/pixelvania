// Reference frames from unmodified pixelfx at a84159688af3ca37a4be05a0d422b94451b2e24e.
// Run by cargo xtask golden. Only this development command needs Node.
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
const [root, out] = process.argv.slice(2);
const load = p => import(pathToFileURL(join(root, p)).href);
const { createFrame, createFx, clearFrame, posed, READY } = await load('src/core/index.js');
const { encodePNG } = await load('tools/lib/png.mjs');
mkdirSync(out, { recursive: true });
const cases = [
 ['impacts/hit-spark', [0,1,2,3,6,12]],
 ['impacts/heavy-landing', [0,1,4,9,18,36]],
 ['melee/sword-slash', [0,1,2,4,8,16]],
 ['melee/hammer-smash', [0,3,5,7,15,36]],
 ['spawn/poof', [0,1,3,8,18,36]],
];
const bytes = words => {const b = Buffer.alloc(words.length * 4); words.forEach((v,i) => b.writeUInt32LE(v,i*4)); return b;};
for (const [id, frames] of cases) {
 const mod = await load(`src/fx/${id}.js`);
 const {size:[w,h],origin:[x,y]} = mod.meta;
 const fx = createFx(mod,{x,y}); const f = createFrame(w,h);
 for(let i=0;i<=Math.max(...frames);i++) {
  if(i) fx.update(1/30);
  if(!frames.includes(i)) continue;
  clearFrame(f); fx.draw(f);
  for(const layer of ['color','light']) writeFileSync(join(out,`${id.replaceAll('/','-')}-${i}-${layer}.png`),encodePNG(w,h,bytes(f[layer])));
 }
}
const slash=await load('src/fx/melee/sword-slash.js');
const hammer=await load('src/fx/melee/hammer-smash.js');
const poses = [{}, READY, slash.act.poses[3][1], hammer.act.poses[2][1], {...READY,yaw:1.2}];
for(let i=0;i<poses.length;i++) {
 const r=posed(poses[i]); const f=createFrame(128,128);
 for(let j=0;j<r.n;j++) {const x=64+r.xs[j], y=100+r.ys[j];if(x>=0&&y>=0&&x<128&&y<128)f.color[y*128+x]=r.color[j];}
 for(const layer of ['color','light']) writeFileSync(join(out,`rig-${i}-${layer}.png`),encodePNG(128,128,bytes(f[layer])));
}
writeFileSync(join(out,'PROVENANCE.txt'),`Source: https://github.com/patch-gg/pixelfx\nCommit: a84159688af3ca37a4be05a0d422b94451b2e24e\nGenerator: cargo xtask golden (Node ${process.version})\nEffects: stock options and stock seeds; createFx.update(1/30), subdivided at 120 Hz.\nRaw RGBA buffers: no actor, background, composite, or tool edge fade.\nRig: stand, ready, sword cut, hammer top, ready yaw 1.2; origin (64,100).\n`);
