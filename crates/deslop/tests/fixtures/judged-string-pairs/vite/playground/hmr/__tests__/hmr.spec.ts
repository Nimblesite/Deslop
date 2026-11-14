  test('accept dep', async () => {
    const el = page.locator('.dep')
    await untilBrowserLogAfter(
      () =>
        editFile('hmrDep.js', (code) =>
          code.replace('const foo = 1', 'const foo = 2 '),
        ),
      [
        '>>> vite:beforeUpdate -- update',
        '(dep) foo was: 1',
        '(dep) foo from dispose: 1',
        '(single dep) foo is now: 2',
        '(single dep) nested foo is now: 1',
        '(multi deps) foo is now: 2',
        '(multi deps) nested foo is now: 1',
        isBundledDev
          ? '[vite] hot updated: playground-temp/hmr/hmrDep.js via playground-temp/hmr/hmr.ts'
          : '[vite] hot updated: /hmrDep.js via /hmr.ts',
        '>>> vite:afterUpdate -- update',
      ],
      true,
    )
    await expect.poll(() => el.textContent()).toMatch('2')

    await untilBrowserLogAfter(
      () =>
        editFile('hmrDep.js', (code) =>
          code.replace('const foo = 2', 'const foo = 3 '),
        ),
      [
        '>>> vite:beforeUpdate -- update',
        '(dep) foo was: 2',
        '(dep) foo from dispose: 2',
        '(single dep) foo is now: 3',
        '(single dep) nested foo is now: 1',
        '(multi deps) foo is now: 3',
        '(multi deps) nested foo is now: 1',
        isBundledDev
          ? '[vite] hot updated: playground-temp/hmr/hmrDep.js via playground-temp/hmr/hmr.ts'
          : '[vite] hot updated: /hmrDep.js via /hmr.ts',
        '>>> vite:afterUpdate -- update',
      ],
      true,
    )
    await expect.poll(() => el.textContent()).toMatch('3')
  })
