<?php

function accumulate(array $values, int $floor): int
{
    $total = 0;
    foreach ($values as $value) {
        if ($value > $floor) {
            $total = $total + $value * 2;
        } else {
            $total = $total - 1;
        }
    }

    return $total;
}
