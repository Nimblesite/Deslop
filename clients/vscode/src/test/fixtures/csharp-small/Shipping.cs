public class Shipping
{
    public decimal BaseRate(decimal weight)
    {
        return weight * 3;
    }

    public Quote QuoteStandard(Parcel parcel)
    {
        var service = "standard";
        var surcharge = 5;
        var quote = new Quote(service);
        quote.SetWeight(parcel.Weight);
        quote.SetRate(BaseRate(parcel.Weight), surcharge);
        quote.Insure(parcel.Value);
        quote.Finalise();
        return quote;
    }

    public Quote QuoteExpress(Parcel parcel)
    {
        var service = "express";
        var surcharge = 12;
        var quote = new Quote(service);
        quote.SetWeight(parcel.Weight);
        quote.SetRate(BaseRate(parcel.Weight), surcharge);
        quote.Insure(parcel.Value);
        quote.Finalise();
        return quote;
    }
}
