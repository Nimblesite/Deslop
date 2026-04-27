namespace Eta
{
    public class Beacon
    {
        public int Tally(int bound)
        {
            if (bound < 0)
            {
                return 0;
            }
            int total = 0;
            for (int step = 0; step < bound; step = step + 1)
            {
                total = total + step;
            }
            return total;
        }
    }
}
