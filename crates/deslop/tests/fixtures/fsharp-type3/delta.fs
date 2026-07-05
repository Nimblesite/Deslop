module Delta

let aggregate (bound: int) : int =
    if bound < 0 then
        0
    else
        let mutable running = 0
        for step in 0 .. bound do
            running <- running + step
            running <- running + 2
        running
