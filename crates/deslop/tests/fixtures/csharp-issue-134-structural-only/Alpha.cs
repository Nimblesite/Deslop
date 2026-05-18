namespace AlphaScaffolding
{
    public class FirstHandler
    {
        public int ProcessFirst(int firstInput, int firstThreshold, int firstOffset)
        {
            if (firstInput < firstThreshold)
            {
                return firstOffset;
            }
            int firstAccumulator = firstOffset;
            int firstScratch = firstThreshold;
            for (int firstStep = firstOffset; firstStep < firstInput; firstStep = firstStep + 1)
            {
                firstAccumulator = firstAccumulator + firstStep;
                firstScratch = firstScratch + firstStep;
                firstAccumulator = firstAccumulator + firstScratch;
            }
            int firstResult = firstAccumulator + firstScratch;
            firstResult = firstResult + firstThreshold;
            firstResult = firstResult + firstOffset;
            return firstResult;
        }
    }
}
