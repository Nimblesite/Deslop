  test('replaces require call in minified code that uses comma operator 2', () => {
    const code = 'require("pure_css_chunk.js"),console.log();'
    const replacer = getEmptyChunkReplacer(['pure_css_chunk.js'], 'cjs')
    const newCode = replacer(code)
    expect(newCode.length).toBe(code.length)
    expect(newCode).toMatchInlineSnapshot(
      `"/* empty css               */console.log();"`,
    )
    expect(newCode).not.toContain('pure_css_chunk.js')
  })

  test('replaces require call in minified code that uses comma operator followed by assignment', () => {
    const code =
      'require("some-module"),require("pure_css_chunk.js");const v=require("other-module");'

    const replacer = getEmptyChunkReplacer(['pure_css_chunk.js'], 'cjs')
    const newCode = replacer(code)
    expect(newCode.length).toBe(code.length)
    expect(newCode).toMatchInlineSnapshot(
      `"require("some-module");/* empty css               */const v=require("other-module");"`,
    )
    expect(newCode).not.toContain('pure_css_chunk.js')
  })
