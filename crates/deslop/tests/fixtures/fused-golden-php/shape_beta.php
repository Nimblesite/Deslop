<?php

class BillingDescriptor
{
    public string $region = 'beta-billing';
    public int $attempts = 9;
    public string $origin = 'https://beta.example.com/billing';

    public function assemble(string $account): string
    {
        return $this->origin . '/' . $account . '/' . $this->region . '?attempts=beta';
    }
}
