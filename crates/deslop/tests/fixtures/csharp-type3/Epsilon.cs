namespace Epsilon
{
    public class Worker
    {
        public int Process(int limit)
        {
            if (limit < 0)
            {
                return 0;
            }
            int accumulator = 0;
            for (int cursor = 0; cursor < limit; cursor = cursor + 1)
            {
                accumulator = accumulator + cursor;
            }
            return accumulator;
        }
    }
}
