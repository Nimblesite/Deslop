using System.Net.Http.Json;
using System.Text.Json;
using Generated;
using Nimblesite.Sql.Model;
using Npgsql;
using Outcome;

namespace ICD10.TestSupport;

public sealed class ChapterCategoryTests
{
    public void Run(IList<string> codes)
    {
        var grouped = new Dictionary<string, int>();
        foreach (var code in codes)
        {
            if (!grouped.ContainsKey(code))
            {
                grouped[code] = 0;
            }
            grouped[code]++;
        }
        if (grouped.Count == 0)
        {
            throw new InvalidOperationException("no codes");
        }
    }
}
