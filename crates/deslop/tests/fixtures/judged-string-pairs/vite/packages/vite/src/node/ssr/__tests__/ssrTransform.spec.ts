test('do not rewrite when function declaration is in scope', async () => {
  const result = await ssrTransformSimple(
    `import { fn } from 'vue';function A(){ function fn() {}; return { fn }; }`,
  )
  expect(result?.code).toMatchInlineSnapshot(
    `
    "const __vite_ssr_import_0__ = await __vite_ssr_import__("vue", {"importedNames":["fn"]});
    function A(){ function fn() {}; return { fn }; }"
  `,
  )
  expect(result?.deps).toEqual(['vue'])
})

// #16452
test('do not rewrite when function expression is in scope', async () => {
  const result = await ssrTransformSimple(
    `import {fn} from './vue';var a = function() { return function fn() { console.log(fn) } }`,
  )
  expect(result?.code).toMatchInlineSnapshot(
    `
    "const __vite_ssr_import_0__ = await __vite_ssr_import__("./vue", {"importedNames":["fn"]});
    var a = function() { return function fn() { console.log(fn) } }"
  `,
  )
})

// #16452
test('do not rewrite when function expression is in global scope', async () => {
  const result = await ssrTransformSimple(
    `import {fn} from './vue';foo(function fn(a = fn) { console.log(fn) })`,
  )
  expect(result?.code).toMatchInlineSnapshot(
    `
    "const __vite_ssr_import_0__ = await __vite_ssr_import__("./vue", {"importedNames":["fn"]});
    foo(function fn(a = fn) { console.log(fn) })"
  `,
  )
})

test('do not rewrite when class declaration is in scope', async () => {
  const result = await ssrTransformSimple(
    `import { cls } from 'vue';function A(){ class cls {} return { cls }; }`,
  )
  expect(result?.code).toMatchInlineSnapshot(
    `
    "const __vite_ssr_import_0__ = await __vite_ssr_import__("vue", {"importedNames":["cls"]});
    function A(){ class cls {} return { cls }; }"
  `,
  )
  expect(result?.deps).toEqual(['vue'])
})

test('do not rewrite when class expression is in scope', async () => {
  const result = await ssrTransformSimple(
    `import { cls } from './vue';var a = function() { return class cls { constructor() { console.log(cls) } } }`,
  )
  expect(result?.code).toMatchInlineSnapshot(
    `
    "const __vite_ssr_import_0__ = await __vite_ssr_import__("./vue", {"importedNames":["cls"]});
    var a = function() { return class cls { constructor() { console.log(cls) } } }"
  `,
  )
})

test('do not rewrite when class expression is in global scope', async () => {
  const result = await ssrTransformSimple(
    `import { cls } from './vue';foo(class cls { constructor() { console.log(cls) } })`,
  )
  expect(result?.code).toMatchInlineSnapshot(
    `
    "const __vite_ssr_import_0__ = await __vite_ssr_import__("./vue", {"importedNames":["cls"]});
    foo(class cls { constructor() { console.log(cls) } })"
  `,
  )
})
