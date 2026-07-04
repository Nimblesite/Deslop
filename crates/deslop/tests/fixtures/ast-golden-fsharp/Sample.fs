// dropped line comment
/// dropped xml doc comment
module Sample

(* dropped block comment *)
let probe (count: int) : float =
    let label = "text"
    let ratio = 3.5
    let flag = true
    let glyph = 'x'
    let hex = 0xFF
    let nothing = ()
    if flag then
        ratio * float count
    else
        0.0
