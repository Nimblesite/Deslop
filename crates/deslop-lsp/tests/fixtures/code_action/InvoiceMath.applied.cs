public class InvoiceMath
{
    private static object ExtractedFromCluster_b50f11(object amounts /* TODO: deslop — fix type */, object taxRate /* TODO: deslop — fix type */) // TODO: deslop — fix return type
    {
        var total = 0;
        foreach (var amount in amounts)
        {
            var taxed = amount * taxRate / 100;
            total += amount + taxed;
        }
        if (total < 0)
        {
            total = 0;
        }
        return total;
    }

    public int TotalWithTax(int[] amounts, int taxRate)
    {
        ExtractedFromCluster_b50f11(amounts, taxRate);
    }

    public int SubtotalWithTax(int[] amounts, int taxRate)
    {
        ExtractedFromCluster_b50f11(amounts, taxRate);
    }
}
