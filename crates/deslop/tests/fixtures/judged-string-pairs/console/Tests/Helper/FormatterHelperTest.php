<?php

class Formatterhelpertest extends TestCase
{
    public function testFormatBlockWithDiacriticLetters()
    {
        $formatter = new FormatterHelper();

        $this->assertEquals(
            '<error>                       </error>'."\n".
            '<error>  Du texte à afficher  </error>'."\n".
            '<error>                       </error>',
            $formatter->formatBlock('Du texte à afficher', 'error', true),
            '::formatBlock() formats a message in a block'
        );
    }

    public function testFormatBlockWithDoubleWidthDiacriticLetters()
    {
        $formatter = new FormatterHelper();
        $this->assertEquals(
            '<error>                    </error>'."\n".
            '<error>  表示するテキスト  </error>'."\n".
            '<error>                    </error>',
            $formatter->formatBlock('表示するテキスト', 'error', true),
            '::formatBlock() formats a message in a block'
        );
    }
}
