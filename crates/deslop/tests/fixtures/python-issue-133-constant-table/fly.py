"""PostgreSQL / DataProvider schema introspection SQL."""

_PUBLIC_FUNCTIONS_SQL = """
SELECT n.nspname AS schema, p.proname AS name
FROM pg_proc p
JOIN pg_namespace n ON n.oid = p.pronamespace
WHERE n.nspname = 'public'
ORDER BY p.proname
"""

_RLS_TABLES_SQL = """
SELECT schemaname, tablename
FROM pg_tables
WHERE rowsecurity = true
ORDER BY tablename
"""

_POLICIES_SQL = """
SELECT schemaname, tablename, policyname, cmd
FROM pg_policies
ORDER BY tablename, policyname
"""

_BUNDLE_CONSTRAINTS_SQL = """
SELECT conname, contype, conrelid::regclass AS table_name
FROM pg_constraint
WHERE connamespace = 'public'::regnamespace
ORDER BY conname
"""

_BUNDLE_COLUMNS_SQL = """
SELECT table_name, column_name, data_type, is_nullable
FROM information_schema.columns
WHERE table_schema = 'public'
ORDER BY table_name, ordinal_position
"""
