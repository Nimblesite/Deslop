// Excerpt of Nimblesite/DataProvider PostgresSchemaInspector.cs at 62b3062e:
// two different catalog queries with the same three lines of command setup.
namespace Fixture;

public static class ColumnAudit
{
    public static void AuditColumns(NpgsqlConnection connection, string schemaName, string tableName)
    {
        using var auditCmd = connection.CreateCommand();
        auditCmd.CommandText = """
                SELECT
                    c.column_name,
                    c.data_type,
                    c.is_nullable,
                    c.column_default,
                    c.character_maximum_length,
                    c.numeric_precision,
                    c.numeric_scale,
                    c.udt_name,
                    a.atttypmod
                FROM information_schema.columns c
                LEFT JOIN pg_catalog.pg_namespace n
                    ON n.nspname = c.table_schema
                LEFT JOIN pg_catalog.pg_class cls
                    ON cls.relname = c.table_name AND cls.relnamespace = n.oid
                LEFT JOIN pg_catalog.pg_attribute a
                    ON a.attrelid = cls.oid AND a.attname = c.column_name AND a.attnum > 0
                WHERE c.table_schema = @schema AND c.table_name = @table
                ORDER BY c.ordinal_position
                """;
        auditCmd.Parameters.AddWithValue("@schema", schemaName);
        auditCmd.Parameters.AddWithValue("@table", tableName);
        auditCmd.ExecuteNonQuery();
    }
}
