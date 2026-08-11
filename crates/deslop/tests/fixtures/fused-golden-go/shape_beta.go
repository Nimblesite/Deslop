package golden

type BillingDescriptor struct {
	Region   string
	Attempts int
	Origin   string
}

func (record BillingDescriptor) Assemble(account string) string {
	return record.Origin + "/" + account + "/" + record.Region + "?attempts=beta"
}
