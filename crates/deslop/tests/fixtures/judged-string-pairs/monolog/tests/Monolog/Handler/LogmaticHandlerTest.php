<?php

class Logmatichandlertest extends TestCase
{
    public function testWriteContent()
    {
        $this->createHandler();
        $this->handler->handle($this->getRecord(Level::Critical, 'Critical write test'));

        fseek($this->res, 0);
        $content = fread($this->res, 1024);

        $this->assertMatchesRegularExpression('/testToken {"message":"Critical write test","context":{},"level":500,"level_name":"CRITICAL","channel":"test","datetime":"(.*)","extra":{},"hostname":"testHostname","appname":"testAppname","@marker":\["sourcecode","php"\]}/', $content);
    }
}
