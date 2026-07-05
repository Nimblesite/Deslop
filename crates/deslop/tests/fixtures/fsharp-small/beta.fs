module Beta

let combine (bound: int) : int =
    let mutable sum = 0
    for step in 1 .. bound do
        if step % 2 = 0 then
            sum <- sum + step * 7
        else
            sum <- sum + 4
    sum
