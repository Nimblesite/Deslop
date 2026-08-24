namespace Golden.RenameLean;

public static class FreightAssessor
{
    public static long AssessParcelLevy(long[] parcels, long levyShare)
    {
        long weightTotal = 0;
        foreach (long parcelMass in parcels)
        {
            weightTotal = weightTotal + parcelMass;
        }

        long levyAmount = weightTotal * levyShare;
        long weightBurden = weightTotal + levyAmount;
        return weightBurden;
    }
}
