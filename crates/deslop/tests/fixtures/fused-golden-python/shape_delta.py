class AuditDescriptor:
    ledger = "delta-audit"
    depth = 128
    archive = "https://delta.example.com/audit"

    def render(self, actor):
        return self.archive + "/" + actor + "/" + self.ledger + "?depth=" + str(self.depth)
