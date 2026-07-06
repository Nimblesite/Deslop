#[allow(dead_code)]
pub fn record_batch(items: &[String], label: &str) {
    let mapped: Vec<String> = items.iter().map(|item| format!("{label}: {item}")).collect();
    match mapped.first() {
        Some(first) => log_line(first),
        None => log_line(label),
    }
    log_line(label);
}

pub fn record_retry(items: &[String], label: &str) {
    let mapped: Vec<String> = items.iter().map(|item| format!("{label}: {item}")).collect();
    match mapped.first() {
        Some(first) => log_line(first),
        None => log_line(label),
    }
    log_line(label);
}

fn log_line(message: &str) {
    println!("{message}");
}
