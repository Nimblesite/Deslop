public class ReceiptTotals
{
    public int TotalWithTax(int[] amounts, int taxRate)
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
}
