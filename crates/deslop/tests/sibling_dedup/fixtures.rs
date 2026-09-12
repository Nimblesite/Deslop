//! Source fixtures for sibling occurrence contracts.

use super::*;

/// Writes two C# files that each contain three near-identical `for`
/// loops nested inside a single method. The sibling pass emits window
/// widths 2, 3, … over those three contiguous loops and — without
/// dedup — every window survives as a separate member of the same
/// cluster.
pub(super) fn write_nested_clone_fixture(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let alpha = "namespace Alpha\n\
                 {\n\
                 public class Runner\n\
                 {\n\
                 public int Run(int input)\n\
                 {\n\
                 if (input < 0) { return 0; }\n\
                 int total = 0;\n\
                 for (int i = 0; i < input; i = i + 1) { total = total + i; }\n\
                 int doubled = 0;\n\
                 for (int j = 0; j < input; j = j + 1) { doubled = doubled + j; }\n\
                 int tripled = 0;\n\
                 for (int k = 0; k < input; k = k + 1) { tripled = tripled + k; }\n\
                 return total + doubled + tripled;\n\
                 }\n\
                 }\n\
                 }\n";
    let beta = "namespace Beta\n\
                {\n\
                public class Worker\n\
                {\n\
                public int Work(int limit)\n\
                {\n\
                if (limit < 0) { return 0; }\n\
                int sum = 0;\n\
                for (int a = 0; a < limit; a = a + 1) { sum = sum + a; }\n\
                int twice = 0;\n\
                for (int b = 0; b < limit; b = b + 1) { twice = twice + b; }\n\
                int thrice = 0;\n\
                for (int c = 0; c < limit; c = c + 1) { thrice = thrice + c; }\n\
                return sum + twice + thrice;\n\
                }\n\
                }\n\
                }\n";
    fs::write(dir.join("Alpha.cs"), alpha)?;
    fs::write(dir.join("Beta.cs"), beta)?;
    Ok(())
}

/// Writes a Python fixture under the same path family as the reported
/// NAP runaway (`alembic/versions/003_cascade_delete_config.py`). The
/// two files contain equivalent migration-shaped code, which exercises
/// the sibling-window path without depending on a private checkout.
pub(super) fn write_phantom_occurrence_fixture(dir: &Path) -> Result<()> {
    let alembic_dir = dir.join("alembic").join("versions");
    let tests_dir = dir.join("tests");
    fs::create_dir_all(&alembic_dir)?;
    fs::create_dir_all(&tests_dir)?;
    fs::write(
        alembic_dir.join("003_cascade_delete_config.py"),
        phantom_occurrence_body("upgrade", "rules", "configs"),
    )?;
    fs::write(
        tests_dir.join("test_sandbox_coverage.py"),
        phantom_occurrence_body("exercise", "jobs", "agents"),
    )?;
    Ok(())
}

fn phantom_occurrence_body(function: &str, child: &str, parent: &str) -> String {
    format!(
        "\"\"\"Synthetic cascade-delete migration fixture.\"\"\"\n\
         from alembic import op\n\
         import sqlalchemy as sa\n\
         \n\
         revision = \"003\"\n\
         down_revision = \"002\"\n\
         branch_labels = None\n\
         depends_on = None\n\
         \n\
         \n\
         def {function}():\n\
             config_id = sa.Column(\"config_id\", sa.Integer(), nullable=False)\n\
             op.add_column(\"{child}\", config_id)\n\
             op.create_index(\"ix_{child}_config_id\", \"{child}\", [\"config_id\"])\n\
             op.create_foreign_key(\n\
                 \"fk_{child}_config_id\",\n\
                 \"{child}\",\n\
                 \"{parent}\",\n\
                 [\"config_id\"],\n\
                 [\"id\"],\n\
                 ondelete=\"CASCADE\",\n\
             )\n\
             op.execute(\"UPDATE {child} SET config_id = 1 WHERE config_id IS NULL\")\n\
             op.alter_column(\"{child}\", \"config_id\", nullable=False)\n\
             op.drop_constraint(\"old_{child}_config_id_fkey\", \"{child}\", type_=\"foreignkey\")\n\
             op.create_foreign_key(\n\
                 \"fk_{child}_config_id_strict\",\n\
                 \"{child}\",\n\
                 \"{parent}\",\n\
                 [\"config_id\"],\n\
                 [\"id\"],\n\
                 ondelete=\"CASCADE\",\n\
             )\n\
             op.drop_index(\"ix_{child}_legacy_config\", table_name=\"{child}\")\n"
    )
}
