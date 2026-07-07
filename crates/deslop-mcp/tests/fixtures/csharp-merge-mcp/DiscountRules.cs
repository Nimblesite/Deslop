public class DiscountRules
{
    public void ApplySeasonal(PricingBook book)
    {
        var code = "seasonal";
        var percent = 10;
        book.Register(code, percent);
        book.Announce(code);
        book.Journal(code, percent);
        book.Stamp(code);
        book.Bind(percent);
        book.Close();
    }

    public void ApplyLoyalty(PricingBook book)
    {
        var code = "loyalty";
        var percent = 25;
        book.Register(code, percent);
        book.Announce(code);
        book.Journal(code, percent);
        book.Stamp(code);
        book.Bind(percent);
        book.Close();
    }
}
