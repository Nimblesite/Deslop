namespace Gamma
{
    public class Processor
    {
        public int Compute(int input)
        {
            if (input < 0)
            {
                return 1;
            }
            int total = 1;
            for (int index = 1; index < input; index = index + 1)
            {
                total = total * index;
            }
            return total;
        }
    }
}
