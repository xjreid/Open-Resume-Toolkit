import {createRequire} from 'node:module';
import {resolve} from 'node:path';
const require=createRequire(import.meta.url);
const {chromium}=require('/Users/xavierreid/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const browser=await chromium.launch({headless:true,executablePath:'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'});
try{
 const page=await browser.newPage({viewport:{width:1100,height:900},deviceScaleFactor:2});
 await page.goto(`file://${resolve(import.meta.dirname,'index.html')}?screen=edit`);
 await page.evaluate(()=>document.fonts.ready);
 await page.screenshot({path:resolve(import.meta.dirname,'../review/edit-small-desktop.png')});
 await page.setViewportSize({width:1440,height:1060});
 await page.screenshot({path:resolve(import.meta.dirname,'../screens/edit.png')});
 console.log('Recaptured Edit at 1100px and 1440px after the reviewer’s selector-width fix.');
}finally{await browser.close()}
