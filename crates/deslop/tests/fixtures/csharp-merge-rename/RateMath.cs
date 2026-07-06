public class RateMath
{
    public void TotalWithTax(int[] amounts, int taxRate)
    {
        var total = 0;
        foreach (var amount in amounts)
        {
            var taxed = amount * taxRate / 100;
            total += amount + taxed;
        }
        Record(total);
    }

    public void SubtotalWithTax(int[] amounts, int taxRate)
    {
        var sum = 0;
        foreach (var amount in amounts)
        {
            var levy = amount * taxRate / 100;
            sum += amount + levy;
        }
        Record(sum);
    }

    private void Record(int value)
    {
    }
}
