module OrderTotals

let computeOrderTotal (lines: OrderLine list) (levy: float) (rebate: float) : float =
    let mutable running = 0.0
    for line in lines do
        let entryTotal = line.Price * line.Count
        if line.Discounted then
            running <- running + entryTotal * 0.9
        else
            running <- running + entryTotal
    let withLevy = running * (1.0 + levy)
    let afterRebate = withLevy - rebate
    if afterRebate < 0.0 then 0.0 else afterRebate
