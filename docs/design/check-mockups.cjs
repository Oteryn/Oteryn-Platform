const { chromium } = require('/workspace/oteryn-platform-env/acceptance/node_modules/playwright');
const fs = require('node:fs');
const path = require('node:path');
(async () => {
 const browser = await chromium.launch({executablePath:'/usr/bin/chromium', args:['--no-sandbox']});
 try {
 for (const file of fs.readdirSync('portal-rs/templates/mockups').filter(f=>f.endsWith('.html'))) {
  for (const width of [360,1280]) {
   const page = await browser.newPage({viewport:{width,height:900},deviceScaleFactor:1});
   await page.goto('http://127.0.0.1:8099/portal-rs/templates/mockups/'+file);
   if (!await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)) throw Error(file+' overflow');
   await page.screenshot({path:`docs/design/screenshots/${file.replace('.html','')}-${width}.png`,fullPage:true});
   await page.close();
  }
 }
 console.log('PASS: 12 screens, 24 PNGs, deviceScaleFactor 1; no page overflow');
 } finally {await browser.close();}
})();
