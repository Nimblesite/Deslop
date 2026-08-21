<?php

function calibrateSensorDrift(array $readings, int $gainFactor): int
{
    $driftSum = 0;
    foreach ($readings as $readingValue) {
        $driftSum = $driftSum + $readingValue;
    }

    $gainAdjusted = $driftSum * $gainFactor;
    $driftScore = $driftSum + $gainAdjusted;
    return $driftScore;
}
