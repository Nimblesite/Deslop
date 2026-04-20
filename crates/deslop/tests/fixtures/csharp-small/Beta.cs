namespace Beta
{
    public class Summer
    {
        public int Run(int limit)
        {
            if (limit < 0)
            {
                return 0;
            }
            int accumulator = 0;
            for (int position = 0; position < limit; position = position + 1)
            {
                accumulator = accumulator + position;
            }
            return accumulator;
        }
    }
}
