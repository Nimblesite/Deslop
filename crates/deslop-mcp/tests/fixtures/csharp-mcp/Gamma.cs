namespace Gamma
{
    public class Multiplier
    {
        public int Times(int factor, int rounds)
        {
            int product = 1;
            for (int counter = 0; counter < rounds; counter = counter + 1)
            {
                product = product * factor;
            }
            return product;
        }
    }
}
