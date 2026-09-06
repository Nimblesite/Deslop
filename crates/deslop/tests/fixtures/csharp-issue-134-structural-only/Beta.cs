namespace BetaScaffolding
{
    public class SecondHandler
    {
        public int ProcessSecond(int secondInput, int secondThreshold, int secondOffset)
        {
            if (secondInput < secondThreshold)
            {
                return secondOffset;
            }
            int secondAccumulator = secondOffset;
            int secondScratch = secondThreshold;
            for (int secondStep = secondOffset; secondStep < secondInput; secondStep = secondStep + 1)
            {
                secondAccumulator = secondAccumulator + secondStep;
                secondScratch = secondScratch + secondStep;
                secondAccumulator = secondAccumulator + secondScratch;
            }
            int secondResult = secondAccumulator + secondScratch;
            secondResult = secondResult + secondThreshold;
            secondResult = secondResult + secondOffset;
            return secondResult;
        }
    }
}
