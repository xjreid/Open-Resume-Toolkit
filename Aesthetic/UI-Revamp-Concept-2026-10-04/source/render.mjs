import {createRequire} from 'node:module';
import {mkdirSync, writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
const require=createRequire(import.meta.url);
const {chromium}=require('/Users/xavierreid/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const sharp=require('/Users/xavierreid/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/sharp');
const root=resolve(import.meta.dirname,'..');
const out=resolve(root,'logo');
const sizes=[16,20,24,32,40,48,64,128,256,512,1024];
for(const size of sizes){
 await sharp(resolve(out,'open-folio-app.svg')).resize(size,size).png().toFile(resolve(out,`app-${size}.png`));
}
for(const name of ['mark','reversed','lockup']){
 await sharp(resolve(out,`open-folio-${name}.svg`),{density:384}).resize({width:name==='lockup'?1640:1024}).png().toFile(resolve(out,`open-folio-${name}.png`));
}
const iconset=resolve(out,'OpenFolio.iconset');mkdirSync(iconset,{recursive:true});
for(const size of [16,32,128,256,512])for(const scale of [1,2]){
 await sharp(resolve(out,'open-folio-app.svg')).resize(size*scale,size*scale).png().toFile(resolve(iconset,`icon_${size}x${size}${scale===2?'@2x':''}.png`));
}
const browser=await chromium.launch({headless:true,executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});
const report=[];
try{
 const page=await browser.newPage({viewport:{width:1440,height:1000},deviceScaleFactor:2});
 for(const screen of ['edit','view','keys','data','settings','overlay','logo']){
   const height=screen==='overlay'?1020:screen==='data'?1180:screen==='view'?1080:screen==='edit'?1060:1000;
   const width=screen==='overlay'?700:1440;
   await page.setViewportSize({width,height});
   await page.goto(`file://${resolve(import.meta.dirname,'index.html')}?screen=${screen}`);
   await page.evaluate(()=>document.fonts.ready);
   const path=resolve(root,screen==='logo'?'logo/logo-presentation.png':`screens/${screen}.png`);
   await page.screenshot({path,fullPage:true});
   const layout=await page.evaluate(()=>({width:document.documentElement.clientWidth,bodyWidth:document.body.scrollWidth,font:document.fonts.check('16px Hanken'),scrollAreas:[...document.querySelectorAll('.main,.document-area,.view-stage')].map(x=>({name:x.className,height:x.clientHeight,scroll:x.scrollHeight})),pageErrors:[...document.images].filter(i=>!i.complete||!i.naturalWidth).map(i=>i.src)}));
   report.push({screen,...layout});
 }
 // Native desktop floor: small desktop window; the overlay remains its own width.
 await page.setViewportSize({width:1100,height:900});
 for(const screen of ['edit','data','settings']){
   await page.goto(`file://${resolve(import.meta.dirname,'index.html')}?screen=${screen}`);
   await page.evaluate(()=>document.fonts.ready);
   await page.screenshot({path:resolve(root,`review/${screen}-small-desktop.png`)});
   report.push({screen:`${screen}-small-desktop`,...await page.evaluate(()=>({width:innerWidth,bodyWidth:document.body.scrollWidth}))});
 }
 writeFileSync(resolve(root,'review/layout-report.json'),JSON.stringify(report,null,2));
 console.log(JSON.stringify(report,null,2));
}finally{await browser.close()}
