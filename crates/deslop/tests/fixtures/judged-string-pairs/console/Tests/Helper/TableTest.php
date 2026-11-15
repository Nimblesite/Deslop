<?php

class TableTest extends TestCase
{
    public function testSectionOutput()
    {
        $sections = [];
        $stream = $this->getOutputStream(true);
        $output = new ConsoleSectionOutput($stream->getStream(), $sections, $stream->getVerbosity(), $stream->isDecorated(), new OutputFormatter());
        $table = new Table($output);
        $table
            ->setHeaders(['ISBN', 'Title', 'Author', 'Price'])
            ->setRows([
                ['99921-58-10-7', 'Divine Comedy', 'Dante Alighieri', '9.95'],
            ]);

        $table->render();

        $table->appendRow(['9971-5-0210-0', 'A Tale of Two Cities', 'Charles Dickens', '139.25']);

        $expected =
            <<<TABLE
                +---------------+---------------+-----------------+-------+
                |\033[32m ISBN          \033[39m|\033[32m Title         \033[39m|\033[32m Author          \033[39m|\033[32m Price \033[39m|
                +---------------+---------------+-----------------+-------+
                | 99921-58-10-7 | Divine Comedy | Dante Alighieri | 9.95  |
                +---------------+---------------+-----------------+-------+
                \x1b[5A\x1b[0J+---------------+----------------------+-----------------+--------+
                |\033[32m ISBN          \033[39m|\033[32m Title                \033[39m|\033[32m Author          \033[39m|\033[32m Price  \033[39m|
                +---------------+----------------------+-----------------+--------+
                | 99921-58-10-7 | Divine Comedy        | Dante Alighieri | 9.95   |
                | 9971-5-0210-0 | A Tale of Two Cities | Charles Dickens | 139.25 |
                +---------------+----------------------+-----------------+--------+

                TABLE;

        $this->assertEquals($expected, $this->getOutputContent($output));
    }

    public function testSectionOutputDoesntClearIfTableIsntRendered()
    {
        $sections = [];
        $stream = $this->getOutputStream(true);
        $output = new ConsoleSectionOutput($stream->getStream(), $sections, $stream->getVerbosity(), $stream->isDecorated(), new OutputFormatter());
        $table = new Table($output);
        $table
            ->setHeaders(['ISBN', 'Title', 'Author', 'Price'])
            ->setRows([
                ['99921-58-10-7', 'Divine Comedy', 'Dante Alighieri', '9.95'],
            ]);

        $table->appendRow(['9971-5-0210-0', 'A Tale of Two Cities', 'Charles Dickens', '139.25']);

        $expected =
            <<<TABLE
                +---------------+----------------------+-----------------+--------+
                |\033[32m ISBN          \033[39m|\033[32m Title                \033[39m|\033[32m Author          \033[39m|\033[32m Price  \033[39m|
                +---------------+----------------------+-----------------+--------+
                | 99921-58-10-7 | Divine Comedy        | Dante Alighieri | 9.95   |
                | 9971-5-0210-0 | A Tale of Two Cities | Charles Dickens | 139.25 |
                +---------------+----------------------+-----------------+--------+

                TABLE;

        $this->assertEquals($expected, $this->getOutputContent($output));
    }

    public function testSectionOutputWithoutDecoration()
    {
        $sections = [];
        $stream = $this->getOutputStream();
        $output = new ConsoleSectionOutput($stream->getStream(), $sections, $stream->getVerbosity(), $stream->isDecorated(), new OutputFormatter());
        $table = new Table($output);
        $table
            ->setHeaders(['ISBN', 'Title', 'Author', 'Price'])
            ->setRows([
                ['99921-58-10-7', 'Divine Comedy', 'Dante Alighieri', '9.95'],
            ]);

        $table->render();

        $table->appendRow(['9971-5-0210-0', 'A Tale of Two Cities', 'Charles Dickens', '139.25']);

        $expected =
            <<<TABLE
                +---------------+---------------+-----------------+-------+
                | ISBN          | Title         | Author          | Price |
                +---------------+---------------+-----------------+-------+
                | 99921-58-10-7 | Divine Comedy | Dante Alighieri | 9.95  |
                +---------------+---------------+-----------------+-------+
                +---------------+----------------------+-----------------+--------+
                | ISBN          | Title                | Author          | Price  |
                +---------------+----------------------+-----------------+--------+
                | 99921-58-10-7 | Divine Comedy        | Dante Alighieri | 9.95   |
                | 9971-5-0210-0 | A Tale of Two Cities | Charles Dickens | 139.25 |
                +---------------+----------------------+-----------------+--------+

                TABLE;

        $this->assertEquals($expected, $this->getOutputContent($output));
    }
}
