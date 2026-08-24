<?php

function route(int $weight, int $distance, string $carrier): string
{
    $score = $weight * 3 + $distance;
    if ($score > 900) {
        return $carrier . '-freight';
    }

    if ($score > 400) {
        return $carrier . '-ground';
    }

    return $carrier . '-parcel';
}
