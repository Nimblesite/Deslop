<?php

// dropped comment
class Thing
{
    /** dropped doc comment */
    public static function probe(int $count): float
    {
        $marker = 'label';
        $flag = true;
        $width = 1.5;
        if ($count > 0) {
            return (float) $count;
        }
        return 0.0;
    }
}
