<?php

class AuditDescriptor
{
    public string $ledger = 'delta-audit';
    public int $depth = 128;
    public string $archive = 'https://delta.example.com/audit';

    public function render(string $actor): string
    {
        return $this->archive . '/' . $actor . '/' . $this->ledger . '?depth=delta';
    }
}
