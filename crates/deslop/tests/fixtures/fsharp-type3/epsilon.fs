module Epsilon

let tally (limit: int) : int =
    if limit < 0 then
        0
    else
        let mutable accumulator = 0
        for cursor in 0 .. limit do
            accumulator <- accumulator + cursor
        accumulator
