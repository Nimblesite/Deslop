class BillingDescriptor:
    region = "beta-billing"
    attempts = 9
    origin = "https://beta.example.com/billing"

    def assemble(self, account):
        return self.origin + "/" + account + "/" + self.region + "?attempts=" + str(self.attempts)
