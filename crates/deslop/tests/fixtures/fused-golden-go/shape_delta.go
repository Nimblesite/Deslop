package golden

type AuditDescriptor struct {
	Ledger  string
	Depth   int
	Archive string
}

func (entry AuditDescriptor) Render(actor string) string {
	return entry.Archive + "/" + actor + "/" + entry.Ledger + "?depth=delta"
}
