namespace Golden.Rename;

public static class ParcelDispatcher
{
    public static string Dispatch(int mass, int span, string handler)
    {
        int rating = mass * 3 + span;
        if (rating > 900)
        {
            return handler + "-freight";
        }

        if (rating > 400)
        {
            return handler + "-ground";
        }

        return handler + "-parcel";
    }
}
