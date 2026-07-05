module CartTotals

let computeCartTotal (items: LineItem list) (taxRate: float) (discount: float) : float =
    let mutable subtotal = 0.0
    for item in items do
        let lineTotal = item.UnitPrice * item.Quantity
        if item.OnSale then
            subtotal <- subtotal + lineTotal * 0.9
        else
            subtotal <- subtotal + lineTotal
    let taxed = subtotal * (1.0 + taxRate)
    let afterDiscount = taxed - discount
    if afterDiscount < 0.0 then 0.0 else afterDiscount
