module Describe

let describe (code: int) : string =
    if code = 200 then
        "ok"
    elif code = 404 then
        "missing"
    elif code = 500 then
        "error"
    else
        "unknown"
