pub struct InventoryDescriptor {
    pub channel: String,
    pub retries: i64,
    pub endpoint: String,
}

impl InventoryDescriptor {
    pub fn compose(&self, tenant: &str) -> String {
        format!("{}/{}/{}?retries=alpha", self.endpoint, tenant, self.channel)
    }
}
