namespace Delta
{
    public class Service
    {
        public int Aggregate(int bound)
        {
            if (bound < 0)
            {
                return 0;
            }
            int running = 0;
            for (int step = 0; step < bound; step = step + 1)
            {
                running = running + step;
                running = running + 2;
            }
            return running;
        }
    }
}
