public class OrderFormatter
{
    private static void ExtractedFromCluster_c50880(object orders /* TODO: deslop — fix type */, object prefix /* TODO: deslop — fix type */, object Console /* TODO: deslop — fix type */)
    {
        var lines = orders.Select(order => { return prefix + order.Name; }).ToList();
        foreach (var line in lines)
        {
            Console.WriteLine(line);
        }
        if (int.TryParse(prefix, out var code))
        {
            Console.WriteLine(code);
        }
    }

    public void PrintOrders(List<Order> orders, string prefix)
    {
        ExtractedFromCluster_c50880(orders, prefix, Console);
    }

    public void PrintReceipts(List<Order> orders, string prefix)
    {
        ExtractedFromCluster_c50880(orders, prefix, Console);
    }
}
