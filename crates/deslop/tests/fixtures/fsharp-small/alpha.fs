module Alpha

let accumulate (limit: int) : int =
    let mutable total = 0
    for index in 1 .. limit do
        if index % 2 = 0 then
            total <- total + index * 3
        else
            total <- total + 1
    total
