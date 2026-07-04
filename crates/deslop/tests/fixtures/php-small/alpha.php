<?php

function compute(int $input): int
{
    if ($input < 0) {
        return 0;
    }
    $total = 0;
    for ($index = 0; $index < $input; $index++) {
        $total = $total + $index;
    }
    return $total;
}
