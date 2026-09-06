<?php

namespace Fixture\Cookie;

class SetCookie
{
    /** @var array<string, mixed> */
    private array $data = [];

    /**
     * Get the cookie name.
     */
    public function getName(): ?string
    {
        return $this->data['Name'];
    }

    /**
     * Set the cookie name.
     *
     * @param string $name Cookie name
     */
    public function setName(string $name): void
    {
        $this->data['Name'] = $name;
    }

    /**
     * Get the cookie value.
     */
    public function getValue(): ?string
    {
        return $this->data['Value'];
    }

    /**
     * Set the cookie value.
     *
     * @param string $value Cookie value
     */
    public function setValue(string $value): void
    {
        $this->data['Value'] = $value;
    }
}
