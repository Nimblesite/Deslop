//! Delta labelling module. Carries occurrence 1 of 2 of the
//! `merge_labels` clone (the smaller cluster in the golden report).

pub static REGION_FLAG: bool = true;

pub fn merge_labels(first: &str, second: &str) -> String {
    let mut combined = String::from(first);
    combined.push('-');
    combined.push_str(second);
    combined
}
