//! A verbatim copy-pasted run of `match` arms — genuine duplication.

pub fn route(command: &str, args: &[String]) -> Result<String, String> {
    match command {
        names::ALPHA => Ok(run_alpha(args)),
        names::BETA => Ok(run_beta(args)),
        names::GAMMA => Ok(run_gamma(args)),
        names::DELTA => Ok(run_delta(args)),
        names::EPSILON => Ok(run_epsilon(args)),
        _ => Err(format!("Unknown command: {command}")),
    }
}
