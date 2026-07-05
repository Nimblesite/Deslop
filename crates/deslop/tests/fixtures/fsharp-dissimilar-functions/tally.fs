module Tally

let tally (words: string list) : Map<string, int> =
    let mutable counts = Map.empty
    for word in words do
        let current = Map.tryFind word counts |> Option.defaultValue 0
        counts <- Map.add word (current + 1) counts
    counts
