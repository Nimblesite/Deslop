pub struct AuditDescriptor {
    pub ledger: String,
    pub depth: i64,
    pub archive: String,
}

impl AuditDescriptor {
    pub fn render(&self, actor: &str) -> String {
        format!("{}/{}/{}?depth=delta", self.archive, actor, self.ledger)
    }
}
