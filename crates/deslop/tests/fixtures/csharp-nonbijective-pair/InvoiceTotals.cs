// Two liftable duplicate methods in one class. The identifier
// substitution between them is deliberately NON-bijective: `Amount`
// aligns with `Price` in one position and with `Cost` in another, and
// `Quantity` aligns with `Units` in one and `Count` in another. That is
// what makes `ContentEvidence::identifiers_vary` true here.
//
// Neither method is scaffolding. Each carries a loop, an accumulator, a
// branch and arithmetic, and the pair is exactly what a parameterised
// extraction lifts. Each fingerprint window covers ONE method, so this
// is not a sibling-declaration family and must never be suppressed as
// one ([RANK-STRUCTURAL-ONLY]).
public class InvoiceTotals
{
    public decimal SummariseDomestic(IReadOnlyList<DomesticLine> lines)
    {
        decimal running = 0m;
        foreach (var line in lines)
        {
            running += line.Amount * line.Quantity;
            if (running > 5000m)
            {
                running -= line.Amount * line.Quantity;
            }
        }
        return Math.Round(running, 2);
    }

    public decimal SummariseExport(IReadOnlyList<ExportRow> rows)
    {
        decimal accrued = 0m;
        foreach (var row in rows)
        {
            accrued += row.Price * row.Units;
            if (accrued > 5000m)
            {
                accrued -= row.Cost * row.Count;
            }
        }
        return Math.Round(accrued, 2);
    }
}
