<?php

function dispatch(int $mass, int $span, string $handler): string
{
    $rating = $mass * 3 + $span;
    if ($rating > 900) {
        return $handler . '-freight';
    }

    if ($rating > 400) {
        return $handler . '-ground';
    }

    return $handler . '-parcel';
}
