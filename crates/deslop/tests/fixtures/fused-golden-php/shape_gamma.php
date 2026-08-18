<?php

class TelemetryDescriptor
{
    public string $stream = 'gamma-telemetry';
    public int $window = 47;
    public string $sink = 'https://gamma.example.com/telemetry';

    public function build(string $device): string
    {
        return $this->sink . '/' . $device . '/' . $this->stream . '?window=gamma';
    }
}
