struct Thing<'a, T> {
    name: &'a str,
    value: T,
}

impl<'a, T> Thing<'a, T> {
    async fn probe(&self, count: i64) -> f64 {
        let marker: char = 'x';
        let flag: bool = true;
        let width: f64 = 1.5;
        let label = format!("{}", self.name);
        match count {
            0 => 0.0,
            _ => count as f64,
        }
    }
}
