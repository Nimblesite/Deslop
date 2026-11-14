<?php

class Inputoptiontest extends TestCase
{
    public function testModes()
    {
        $option = new InputOption('foo', 'f');
        $this->assertFalse($option->acceptValue(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');
        $this->assertFalse($option->isValueRequired(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');
        $this->assertFalse($option->isValueOptional(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');
        $this->assertFalse($option->isDeprecated(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');
        $this->assertFalse($option->isHidden(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');

        $option = new InputOption('foo', 'f', null);
        $this->assertFalse($option->acceptValue(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');
        $this->assertFalse($option->isValueRequired(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');
        $this->assertFalse($option->isValueOptional(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');
        $this->assertFalse($option->isDeprecated(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');
        $this->assertFalse($option->isHidden(), '__construct() gives a "InputOption::VALUE_NONE" mode by default');

        $option = new InputOption('foo', 'f', InputOption::VALUE_NONE);
        $this->assertFalse($option->acceptValue(), '__construct() can take "InputOption::VALUE_NONE" as its mode');
        $this->assertFalse($option->isValueRequired(), '__construct() can take "InputOption::VALUE_NONE" as its mode');
        $this->assertFalse($option->isValueOptional(), '__construct() can take "InputOption::VALUE_NONE" as its mode');
        $this->assertFalse($option->isDeprecated(), '__construct() can take "InputOption::VALUE_NONE" as its mode');
        $this->assertFalse($option->isHidden(), '__construct() can take "InputOption::VALUE_NONE" as its mode');

        $option = new InputOption('foo', 'f', InputOption::VALUE_REQUIRED);
        $this->assertTrue($option->acceptValue(), '__construct() can take "InputOption::VALUE_REQUIRED" as its mode');
        $this->assertTrue($option->isValueRequired(), '__construct() can take "InputOption::VALUE_REQUIRED" as its mode');
        $this->assertFalse($option->isValueOptional(), '__construct() can take "InputOption::VALUE_REQUIRED" as its mode');
        $this->assertFalse($option->isDeprecated(), '__construct() can take "InputOption::VALUE_REQUIRED" as its mode');
        $this->assertFalse($option->isHidden(), '__construct() can take "InputOption::VALUE_REQUIRED" as its mode');

        $option = new InputOption('foo', 'f', InputOption::VALUE_OPTIONAL);
        $this->assertTrue($option->acceptValue(), '__construct() can take "InputOption::VALUE_OPTIONAL" as its mode');
        $this->assertFalse($option->isValueRequired(), '__construct() can take "InputOption::VALUE_OPTIONAL" as its mode');
        $this->assertTrue($option->isValueOptional(), '__construct() can take "InputOption::VALUE_OPTIONAL" as its mode');
        $this->assertFalse($option->isDeprecated(), '__construct() can take "InputOption::VALUE_OPTIONAL" as its mode');
        $this->assertFalse($option->isHidden(), '__construct() can take "InputOption::VALUE_OPTIONAL" as its mode');

        $option = new InputOption('foo', 'f', InputOption::DEPRECATED);
        $this->assertFalse($option->acceptValue(), '__construct() can take "InputOption::DEPRECATED" as its mode');
        $this->assertFalse($option->isValueRequired(), '__construct() can take "InputOption::DEPRECATED" as its mode');
        $this->assertFalse($option->isValueOptional(), '__construct() can take "InputOption::DEPRECATED" as its mode');
        $this->assertTrue($option->isDeprecated(), '__construct() can take "InputOption::DEPRECATED" as its mode');
        $this->assertFalse($option->isHidden(), '__construct() can take "InputOption::DEPRECATED" as its mode');

        $option = new InputOption('foo', 'f', InputOption::HIDDEN);
        $this->assertFalse($option->acceptValue(), '__construct() can take "InputOption::HIDDEN" as its mode');
        $this->assertFalse($option->isValueRequired(), '__construct() can take "InputOption::HIDDEN" as its mode');
        $this->assertFalse($option->isValueOptional(), '__construct() can take "InputOption::HIDDEN" as its mode');
        $this->assertFalse($option->isDeprecated(), '__construct() can take "InputOption::HIDDEN" as its mode');
        $this->assertTrue($option->isHidden(), '__construct() can take "InputOption::HIDDEN" as its mode');
    }
}
