class InventoryDescriptor:
    channel = "alpha-inventory"
    retries = 3
    endpoint = "https://alpha.example.com/inventory"

    def compose(self, tenant):
        return self.endpoint + "/" + tenant + "/" + self.channel + "?retries=" + str(self.retries)
