fn merged_from_cluster_600951(book: &mut Vec<(String, i64)>, arg0: &'static str, arg1: i64) {
    let label = arg0;
    let ceiling = arg1;
    book.push((label.to_owned(), ceiling));
    book.push((label.to_uppercase(), ceiling * 2));
    book.push((label.to_lowercase(), ceiling + 7));
    book.push((label.trim().to_owned(), ceiling - 1));
    book.push((label.repeat(2), ceiling / 2));
    book.sort();
}

pub fn apply_standard(book: &mut Vec<(String, i64)>) {
    merged_from_cluster_600951(book, "standard", 100);
}

pub fn apply_premium(book: &mut Vec<(String, i64)>) {
    merged_from_cluster_600951(book, "premium", 250);
}
