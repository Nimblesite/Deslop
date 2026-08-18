<?php

class InventoryDescriptor
{
    public string $channel = 'alpha-inventory';
    public int $retries = 3;
    public string $endpoint = 'https://alpha.example.com/inventory';

    public function compose(string $tenant): string
    {
        return $this->endpoint . '/' . $tenant . '/' . $this->channel . '?retries=alpha';
    }
}
