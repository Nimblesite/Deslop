import {expect,test} from '@playwright/test';

// [SITE-NAV-FONTS] Navigation icons must render without a third-party font service.
test('navigation icons load when external font services are unavailable',async({page})=>{
  await page.route('https://fonts.googleapis.com/**',route=>route.abort());
  await page.route('https://fonts.gstatic.com/**',route=>route.abort());
  await page.goto('/docs/');
  const loaded=await page.evaluate(async()=>{
    try {return (await document.fonts.load('16px "Material Symbols Outlined"')).length;}
    catch {return 0;}
  });
  expect(loaded).toBeGreaterThan(0);
  await expect(page.getByRole('navigation',{name:'Documentation'}).getByRole('link',{name:'Getting Started'})).toBeVisible();
});
