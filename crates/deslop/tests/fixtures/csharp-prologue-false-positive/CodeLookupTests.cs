using System.Net.Http.Json;
using System.Text.Json;
using Generated;
using Nimblesite.Sql.Model;
using Npgsql;
using Outcome;

namespace ICD10.TestSupport;

public class CodeLookupTests
{
    private readonly HttpClient _client;

    public CodeLookupTests(HttpClient client)
    {
        _client = client;
    }

    public async Task LookupOk()
    {
        var response = await _client.GetAsync("/api/codes/A00.0");
        response.EnsureSuccessStatusCode();
        var body = await response.Content.ReadAsStringAsync();
        if (string.IsNullOrEmpty(body))
        {
            throw new InvalidOperationException("empty body");
        }
    }
}
