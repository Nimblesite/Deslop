namespace Golden.Verbatim;

public static class WeightedTotals
{
    public static int Accumulate(int[] values, int floor)
    {
        int total = 0;
        foreach (int value in values)
        {
            if (value > floor)
            {
                total = total + value * 2;
            }
            else
            {
                total = total - 1;
            }
        }

        return total;
    }
}
