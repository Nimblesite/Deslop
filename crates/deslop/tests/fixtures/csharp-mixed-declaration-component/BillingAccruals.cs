// A MIXED same-shape component in one class.
//
// `AccrueDomestic` and `AccrueRegional` are a real Type-2 clone pair:
// every literal agrees and the identifier substitution is a consistent
// 1:1 rename (running->total, line->row, lines->rows). Lifting one into
// the other is a straight parameterless extraction.
//
// `AccrueExport` is the divergent sibling: different literals (250m, 4)
// and a NON-bijective identifier mapping (Amount aligns with both Price
// and Cost). It alone is what cluster-wide substance evidence measures.
//
// All three collapse to the same skeleton, so they arrive as one
// component. The divergent third member must not convict the first two
// ([RANK-STRUCTURAL-ONLY]): no window here covers more than one
// declaration, so nothing in this file is a sibling-declaration family.
public class BillingAccruals
{
    public decimal AccrueDomestic(IReadOnlyList<DomesticLine> lines)
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

    public decimal AccrueRegional(IReadOnlyList<RegionalLine> rows)
    {
        decimal total = 0m;
        foreach (var row in rows)
        {
            total += row.Amount * row.Quantity;
            if (total > 5000m)
            {
                total -= row.Amount * row.Quantity;
            }
        }
        return Math.Round(total, 2);
    }

    public decimal AccrueExport(IReadOnlyList<ExportRow> entries)
    {
        decimal carried = 0m;
        foreach (var entry in entries)
        {
            carried += entry.Price * entry.Units;
            if (carried > 250m)
            {
                carried -= entry.Cost * entry.Count;
            }
        }
        return Math.Round(carried, 4);
    }
}
