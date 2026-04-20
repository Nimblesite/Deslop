namespace Mixed
{
    public class Library
    {
        public int Alpha(int value)
        {
            if (value < 0)
            {
                return 0;
            }
            int total = 0;
            for (int index = 0; index < value; index = index + 1)
            {
                total = total + index;
            }
            return total;
        }

        public int Beta(int bound)
        {
            if (bound < 0)
            {
                return 0;
            }
            int running = 0;
            for (int step = 0; step < bound; step = step + 1)
            {
                running = running + step;
            }
            return running;
        }
    }
}
