using System.Net.Http.Json;
using System.Text.Json;
using Generated;
using Nimblesite.Sql.Model;
using Npgsql;
using Outcome;

namespace ICD10.TestSupport;

public static class Icd10TestDatabase
{
    public static void Initialize(string connectionString, string schemaYamlPath)
    {
        if (!File.Exists(schemaYamlPath))
        {
            throw new FileNotFoundException(
                $"icd10-schema.yaml not found at '{schemaYamlPath}'",
                schemaYamlPath
            );
        }

        using var conn = new NpgsqlConnection(connectionString);
        conn.Open();

        var schema = SchemaYamlSerializer.FromYamlFile(schemaYamlPath);
        PostgresDdlGenerator.MigrateSchema(conn, schema);
    }
}
