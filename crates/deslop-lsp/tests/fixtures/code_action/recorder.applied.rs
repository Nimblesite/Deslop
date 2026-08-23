// TODO: deslop — replace `DeslopTodo` with real types.
type DeslopTodo = ();

fn extracted_from_cluster_edefa3(items: DeslopTodo, label: DeslopTodo) -> DeslopTodo {
    let mapped: Vec<String> = items.iter().map(|item| format!("{label}: {item}")).collect();
    match mapped.first() {
        Some(first) => log_line(first),
        None => log_line(label),
    }
    log_line(label);
}

#[allow(dead_code)]
pub fn record_batch(items: &[String], label: &str) {
    extracted_from_cluster_edefa3(items, label);
}

pub fn record_retry(items: &[String], label: &str) {
    extracted_from_cluster_edefa3(items, label);
}

fn log_line(message: &str) {
    println!("{message}");
}
