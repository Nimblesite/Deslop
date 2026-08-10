pub struct BillingDescriptor {
    pub region: String,
    pub attempts: i64,
    pub origin: String,
}

impl BillingDescriptor {
    pub fn assemble(&self, account: &str) -> String {
        format!("{}/{}/{}?attempts=beta", self.origin, account, self.region)
    }
}
