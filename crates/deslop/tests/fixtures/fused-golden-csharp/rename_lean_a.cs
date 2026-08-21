namespace Golden.RenameLean;

public static class SensorCalibrator
{
    public static long CalibrateSensorDrift(long[] readings, long gainFactor)
    {
        long driftSum = 0;
        foreach (long readingValue in readings)
        {
            driftSum = driftSum + readingValue;
        }

        long gainAdjusted = driftSum * gainFactor;
        long driftScore = driftSum + gainAdjusted;
        return driftScore;
    }
}
