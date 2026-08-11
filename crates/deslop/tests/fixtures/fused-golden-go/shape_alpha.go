package golden

type InventoryDescriptor struct {
	Channel  string
	Retries  int
	Endpoint string
}

func (descriptor InventoryDescriptor) Compose(tenant string) string {
	return descriptor.Endpoint + "/" + tenant + "/" + descriptor.Channel + "?retries=alpha"
}
