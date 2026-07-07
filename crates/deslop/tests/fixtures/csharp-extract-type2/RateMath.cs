public class RateMath
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

    public int SubtotalWithTax(int[] amounts, int taxRate)
    {
        var sum = 0;
        foreach (var amount in amounts)
        {
            var levy = amount * taxRate / 100;
            sum += amount + levy;
        }
        if (sum < 0)
        {
            sum = 0;
        }
        return sum;
    }
}
