module BillingB

open System
open System.Collections.Generic

let totalEligible (values: int list) =
    let mutable total = 0
    for value in values do
        if value > 0 then
            let adjusted = value * 3
            total <- total + adjusted
        else
            total <- total + 1
    total
