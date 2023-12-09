--TEST--
Test FluentBundle::hasMessage
--FILE--
<?php

$resource = <<<'FTL'
    hello = Hello, world!
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);
    $response = $bundle->hasMessage('hello');
    var_dump($response);
    $response = $bundle->hasMessage('hello-world');
    var_dump($response);
} catch (\Exception $e) {
    print_r($e);
}
?>
===DONE===
--EXPECT--
bool(true)
bool(false)
===DONE===
