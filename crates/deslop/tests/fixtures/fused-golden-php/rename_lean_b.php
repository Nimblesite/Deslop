<?php

function assessParcelLevy(array $parcels, int $levyShare): int
{
    $weightTotal = 0;
    foreach ($parcels as $parcelMass) {
        $weightTotal = $weightTotal + $parcelMass;
    }

    $levyAmount = $weightTotal * $levyShare;
    $weightBurden = $weightTotal + $levyAmount;
    return $weightBurden;
}
