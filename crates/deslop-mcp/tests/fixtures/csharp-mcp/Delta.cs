namespace Delta
{
    public class Multiplier
    {
        public int Times(int factor, int rounds)
        {
            int product = 1;
            for (int counter = 1; counter < rounds; counter = counter + 1)
            {
                product = product * factor;
            }
            return product;
        }
    }
}
