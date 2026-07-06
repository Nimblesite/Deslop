public class OrderFormatter
{
    public void PrintOrders(List<Order> orders, string prefix)
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

    public void PrintReceipts(List<Order> orders, string prefix)
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
}
