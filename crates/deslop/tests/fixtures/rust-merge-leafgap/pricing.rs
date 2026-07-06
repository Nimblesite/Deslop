pub fn apply_standard(book: &mut Vec<(String, i64)>) {
    let label = "standard";
    let ceiling = 100;
    book.push((label.to_owned(), ceiling));
    book.push((label.to_uppercase(), ceiling * 2));
    book.push((label.to_lowercase(), ceiling + 7));
    book.push((label.trim().to_owned(), ceiling - 1));
    book.push((label.repeat(2), ceiling / 2));
    book.sort();
}

pub fn apply_premium(book: &mut Vec<(String, i64)>) {
    let label = "premium";
    let ceiling = 250;
    book.push((label.to_owned(), ceiling));
    book.push((label.to_uppercase(), ceiling * 2));
    book.push((label.to_lowercase(), ceiling + 7));
    book.push((label.trim().to_owned(), ceiling - 1));
    book.push((label.repeat(2), ceiling / 2));
    book.sort();
}
