//! [MCP-WIRE-FRAMING] A closed child stdout must fail instead of spinning forever.

use crate::read_lsp_frame;
use anyhow::{Context, Result};
use std::{
    io::{self, BufReader},
    process::{Command, Stdio},
};

const CHILD_ARGUMENT: &str = "--help";
const EOF_MESSAGE: &str = "LSP stream ended while reading frame headers";

#[test]
fn closed_child_stdout_returns_unexpected_eof() -> Result<()> {
    let mut child = Command::new(std::env::current_exe()?)
        .arg(CHILD_ARGUMENT)
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().context("child stdout")?;
    assert!(
        child.wait()?.success(),
        "the child must have exited before the read"
    );
    let error = read_lsp_frame(&mut BufReader::new(stdout))
        .err()
        .context("closed stream must fail")?;
    let io_error = error
        .downcast_ref::<io::Error>()
        .context("EOF must retain its I/O error type")?;
    assert_eq!(io_error.kind(), io::ErrorKind::UnexpectedEof);
    assert_eq!(io_error.to_string(), EOF_MESSAGE);
    Ok(())
}
