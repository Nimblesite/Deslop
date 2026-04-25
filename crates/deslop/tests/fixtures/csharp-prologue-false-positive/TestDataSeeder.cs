using System.Net.Http.Json;
using System.Text.Json;
using Generated;
using Nimblesite.Sql.Model;
using Npgsql;
using Outcome;

namespace ICD10.TestSupport;

public static class TestDataSeeder
{
    private const string IcdEmbeddingModel = "MedEmbed-Small-v0.1";

    public static void Seed(NpgsqlConnection conn)
    {
        SeedChaptersAsync(conn).GetAwaiter().GetResult();
        SeedBlocksAsync(conn).GetAwaiter().GetResult();
        SeedCategoriesAsync(conn).GetAwaiter().GetResult();
        SeedCodesAsync(conn).GetAwaiter().GetResult();
        SeedAchiBlocksAsync(conn).GetAwaiter().GetResult();
        SeedAchiCodesAsync(conn).GetAwaiter().GetResult();
    }

    public static void SeedEmbeddings(NpgsqlConnection conn)
        => SeedEmbeddingsAsync(conn).GetAwaiter().GetResult();
}
