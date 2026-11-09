import {expect,test} from '@playwright/test';

// [SITE-LOAD-ANALYTICS] Optional analytics cannot hold the usable page open.
test('the page finishes loading while analytics is unavailable',async({page})=>{
  let release;
  const pending=new Promise(resolve=>{release=resolve;});
  await page.route('https://www.googletagmanager.com/**',async route=>{
    await pending;
    await route.fulfill({body:'',contentType:'application/javascript'});
  });
  try {
    await page.goto('/docs/');
    await expect(page.getByRole('heading',{name:'Getting Started',exact:true})).toBeVisible();
    expect(await page.evaluate(()=>Array.from(window.dataLayer,entry=>entry[0]))).toEqual(['js','config']);
  } finally {release();}
});
