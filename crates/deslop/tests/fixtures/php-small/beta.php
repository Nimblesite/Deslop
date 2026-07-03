<?php

function run(int $limit): int
{
    if ($limit < 0) {
        return 0;
    }
    $accumulator = 0;
    for ($position = 0; $position < $limit; $position++) {
        $accumulator = $accumulator + $position;
    }
    return $accumulator;
}
