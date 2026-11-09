//! [CLONE-NOISE-PY-ASSERT-ONLY] Assertion-only test blocks.
//!
//! Two pytest functions made of nothing but `assert` statements share an AST
//! shape and a token alphabet, yet each checks different concrete paths and
//! values. Across files, a family of them that differs in raw bytes is not
//! duplication; a byte-identical copy of one is. Both halves are pinned in one
//! run, so the suppression cannot pass by going blind (gh #461: this rule had
//! a spec section and production code, but no test named it).

use std::{fs, path::Path};

use crate::common::*;

/// Floors the rule must hold at: the cluster-level default and a tighter one.
const NODE_FLOORS: [u32; 2] = [8, 20];
/// Two unrelated assertion blocks over different objects and values.
const ORDERS: (&str, &str) = (
    "test_orders.py",
    "def test_order_totals(order):\n    assert order.total == 120\n    assert order.currency == \"AUD\"\n    assert order.items[0].sku == \"A-100\"\n    assert order.items[1].quantity == 3\n    assert order.shipping.method == \"express\"\n    assert order.customer.email.endswith(\"@example.com\")\n",
);
const INVOICES: (&str, &str) = (
    "test_invoices.py",
    "def test_invoice_fields(invoice):\n    assert invoice.amount_due == 87\n    assert invoice.status == \"open\"\n    assert invoice.lines[0].code == \"SVC-9\"\n    assert invoice.lines[2].hours == 11\n    assert invoice.payment.terms == \"net-30\"\n    assert invoice.issuer.name.startswith(\"Acme\")\n",
);
/// One assertion block copied byte for byte into two files.
const COPIED_BLOCK: &str = "def test_profile_copy(profile):\n    assert profile.display_name == \"Ada Lovelace\"\n    assert profile.roles[0] == \"admin\"\n    assert profile.settings.theme == \"dark\"\n    assert profile.limits.daily == 500\n    assert profile.limits.monthly == 9000\n    assert profile.region == \"ap-southeast-2\"\n";
const COPY_FILES: [&str; 2] = ["test_copy_a.py", "test_copy_b.py"];
/// The copied block's line span in each file, and the lines it duplicates.
const COPY_SPAN: (u64, u64) = (1, 7);
const COPY_DUPLICATED_LOC: u64 = 14;
/// The scan directory and the metric the copy is counted in.
const SCAN_DIR: &str = "assert_only";
const DUPLICATED_LOC: &str = "duplicated_loc";

/// Writes the four test files into `root`.
fn write_fixture(root: &Path) -> Result<()> {
    for (name, body) in [ORDERS, INVOICES] {
        fs::write(root.join(name), body)?;
    }
    for name in COPY_FILES {
        fs::write(root.join(name), COPIED_BLOCK)?;
    }
    Ok(())
}

#[test]
fn distinct_assertion_blocks_are_not_duplication_but_a_verbatim_copy_is() -> Result<()> {
    let (_tmp, root) = temp_scan_dir(SCAN_DIR)?;
    write_fixture(&root)?;
    for floor in NODE_FLOORS {
        let report = run_report(&root, floor)?;
        let clones = clone_findings(&report);
        let [copy] = clones.as_slice() else {
            return Err(anyhow::anyhow!(
                "at --min-nodes {floor} the only clone is the verbatim copy, got {}: {report:#}",
                clones.len()
            ));
        };
        assert_eq!(
            cluster_kind(copy),
            IDENTICAL_KIND,
            "the copy is byte-identical"
        );
        assert_eq!(
            occurrence_paths(copy),
            COPY_FILES,
            "the copy spans exactly its two files"
        );
        assert_eq!(cluster_line_spans(copy), vec![COPY_SPAN; COPY_FILES.len()]);
        for cluster in &clones {
            for path in occurrence_paths(cluster) {
                assert!(
                    path != ORDERS.0 && path != INVOICES.0,
                    "distinct assertion blocks must not be published as a clone: {cluster:#}"
                );
            }
        }
        assert_eq!(
            metric_field(&report, DUPLICATED_LOC).as_u64(),
            Some(COPY_DUPLICATED_LOC),
            "only the copy's lines count as duplicated at --min-nodes {floor}"
        );
    }
    Ok(())
}
