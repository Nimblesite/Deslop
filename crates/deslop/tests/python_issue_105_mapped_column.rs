//! E2E regression for GH #105 [CLONE-NOISE-PY-MAPPED-COLUMN].
//!
//! `SQLAlchemy` `Mapped[T] = mapped_column(...)` declaration blocks share
//! the same token alphabet (`Mapped`, `mapped_column`, `ForeignKey`,
//! `UUID`, `datetime`) but each block declares a different table's
//! columns. Token Jaccard alone clusters them; structural similarity is
//! ~0 and embeddings agree. The cluster filter must drop those clusters.


use crate::common::*;

#[test]
fn sqlalchemy_mapped_column_blocks_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-105-mapped-column");
    let report = run_report(&scan_root, 4)?;
    let offenders = summaries_where(&report, &scan_root, |text| text.contains("mapped_column("))?;
    assert!(
        offenders.is_empty(),
        "Mapped[T] = mapped_column(...) declaration blocks must not surface \
         as duplicate logic: {offenders:#?}"
    );
    Ok(())
}
