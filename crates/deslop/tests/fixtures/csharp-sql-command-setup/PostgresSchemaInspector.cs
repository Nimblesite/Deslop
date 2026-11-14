// Excerpt of Nimblesite/DataProvider PostgresSchemaInspector.cs at 62b3062e:
// two different catalog queries with the same three lines of command setup.
namespace Fixture;

public static class PostgresSchemaInspector
{
    public static void ReadColumns(NpgsqlConnection connection, string schemaName, string tableName)
    {
        using var colCmd = connection.CreateCommand();
        colCmd.CommandText = """
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
        colCmd.Parameters.AddWithValue("@schema", schemaName);
        colCmd.Parameters.AddWithValue("@table", tableName);
        colCmd.ExecuteNonQuery();
    }

    public static void ReadIndexes(NpgsqlConnection connection, string schemaName, string tableName)
    {
        using var idxCmd = connection.CreateCommand();
        idxCmd.CommandText = """
                SELECT
                    i.relname AS index_name,
                    ix.indisunique AS is_unique,
                    pg_get_indexdef(ix.indexrelid) AS index_def,
                    (SELECT array_agg(a.attname ORDER BY ord.n)
                     FROM unnest(ix.indkey) WITH ORDINALITY AS ord(attnum, n)
                     LEFT JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ord.attnum
                     WHERE ord.attnum > 0) AS column_names,
                    (SELECT bool_or(ord.attnum = 0)
                     FROM unnest(ix.indkey) WITH ORDINALITY AS ord(attnum, n)) AS has_expressions
                FROM pg_class t
                JOIN pg_index ix ON t.oid = ix.indrelid
                JOIN pg_class i ON i.oid = ix.indexrelid
                JOIN pg_namespace n ON n.oid = t.relnamespace
                LEFT JOIN pg_constraint owned_constraint
                    ON owned_constraint.conindid = ix.indexrelid
                    AND owned_constraint.contype IN ('p', 'u')
                WHERE n.nspname = @schema
                AND t.relname = @table
                AND NOT ix.indisprimary
                AND owned_constraint.oid IS NULL
                ORDER BY i.relname
                """;
        idxCmd.Parameters.AddWithValue("@schema", schemaName);
        idxCmd.Parameters.AddWithValue("@table", tableName);
        idxCmd.ExecuteNonQuery();
    }
}
