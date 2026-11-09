import {expect,test} from '@playwright/test';

// [SITE-NAV-FONTS] Navigation icons must render without a third-party font service.
test('navigation icons load when external font services are unavailable',async({page})=>{
  await page.route('https://fonts.googleapis.com/**',route=>route.abort());
  await page.route('https://fonts.gstatic.com/**',route=>route.abort());
  await page.goto('/docs/',{waitUntil:'domcontentloaded'});
  const loaded=await page.evaluate(async()=>{
    try {return (await document.fonts.load('16px "Material Symbols Outlined"')).length;}
    catch {return 0;}
  });
  expect(loaded).toBeGreaterThan(0);
  await expect(page.getByRole('navigation',{name:'Documentation'}).getByRole('link',{name:'Getting Started'})).toBeVisible();
});

test('reading and code fonts load without external font services',async({page})=>{
  await page.route('https://fonts.googleapis.com/**',route=>route.abort());
  await page.route('https://fonts.gstatic.com/**',route=>route.abort());
  await page.goto('/docs/',{waitUntil:'domcontentloaded'});
  for(const family of ['Inter','JetBrains Mono']) {
    const loaded=await page.evaluate(async family=>{
      try {return (await document.fonts.load(`16px "${family}"`)).length;}
      catch {return 0;}
    },family);
    expect(loaded,`${family} is delivered by the website`).toBeGreaterThan(0);
  }
});

const ICON_SIZE=24;
async function publishedIconNames(page) {
  return page.evaluate(async()=>{
    const parse=(text,type)=>new DOMParser().parseFromString(text,type);
    const sitemap=parse(await (await fetch('/sitemap.xml')).text(),'application/xml');
    const routes=[...sitemap.querySelectorAll('loc')].map(node=>new URL(node.textContent).pathname);
    const names=new Set();
    for(const route of routes) {
      const document=parse(await (await fetch(route)).text(),'text/html');
      for(const icon of document.querySelectorAll('.material-symbols-outlined')) names.add(icon.textContent.trim());
    }
    return [...names];
  });
}

test('every published navigation icon retains its ligature after subsetting',async({page})=>{
  await page.goto('/docs/');
  const names=await publishedIconNames(page);
  expect(names.length).toBeGreaterThan(0);
  const widths=await page.evaluate(async({names,size})=>{
    await document.fonts.load(`${size}px "Material Symbols Outlined"`);
    const context=document.createElement('canvas').getContext('2d');
    context.font=`${size}px "Material Symbols Outlined"`;
    return names.map(name=>({name,width:context.measureText(name).width}));
  },{names,size:ICON_SIZE});
  for(const {name,width} of widths) expect(width,name).toBeCloseTo(ICON_SIZE);
});
