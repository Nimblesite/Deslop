//! Slash-command dispatch table (mirrors a real routing `match`).

/// Routes a slash command name to its handler, mirroring the
/// `basilisk-zed/src/logic.rs` dispatch that surfaced GH #176.
pub fn slash_command_output(command: &str, args: &[String]) -> Result<String, String> {
    match command {
        names::PROFILE => Ok(slash_profile(args)),
        names::PROFSTOP => Ok(slash_profstop()),
        names::PROFSNAPSHOT => Ok(slash_profsnapshot()),
        names::MEMLEAK => Ok(slash_memleak()),
        names::MEMSTOP => Ok(slash_memstop()),
        names::MEMREFS => Ok(slash_memrefs(args)),
        names::MODULES => Ok(slash_modules(args)),
        names::SYMBOLS => Ok(slash_symbols(args)),
        names::HEALTH => Ok(slash_health()),
        names::BASILISK => Ok(slash_basilisk()),
        names::TESTS => Ok(slash_tests(args)),
        names::RUNTESTS => Ok(slash_runtests(args)),
        names::TESTFILE => Ok(slash_testfile(args)),
        _ => Err(format!("Unknown slash command: {command}")),
    }
}
