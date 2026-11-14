<?php

class Logentrieshandlertest extends TestCase
{
    public function testWriteContent()
    {
        $this->createHandler();
        $this->handler->handle($this->getRecord(Level::Critical, 'Critical write test'));

        fseek($this->res, 0);
        $content = fread($this->res, 1024);

        $this->assertMatchesRegularExpression('/testToken \[\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6}\+00:00\] test.CRITICAL: Critical write test/', $content);
    }
}
