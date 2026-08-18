namespace Golden.Rename;

public static class ShipmentRouter
{
    public static string Route(int weight, int distance, string carrier)
    {
        int score = weight * 3 + distance;
        if (score > 900)
        {
            return carrier + "-freight";
        }

        if (score > 400)
        {
            return carrier + "-ground";
        }

        return carrier + "-parcel";
    }
}
