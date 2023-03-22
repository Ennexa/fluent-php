<?php

$resource = <<<'FTL'
    hello = Hello, world!
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);
    $response = $bundle->formatPattern('hello', []);
    var_dump($response);
} catch (\Exception $e) {
    print_r($e);
}
