--TEST--
ResourceCache: string-based caching shows hits on repeated fromString
--FILE--
<?php
FluentPHP\ResourceCache::clear();

$resource = <<<'FTL'
    hello = Hello, world!
    FTL;

$r1 = FluentPHP\ResourceCache::fromString($resource);
$r2 = FluentPHP\ResourceCache::fromString($resource);

$bundle = new FluentPHP\FluentBundle('en');
$bundle->addResource($r1);
echo $bundle->formatPattern('hello', []), PHP_EOL;

$stats = FluentPHP\ResourceCache::getStats();
echo "hits: ", $stats['hits'], PHP_EOL;
echo "misses: ", $stats['misses'], PHP_EOL;
echo "loads: ", $stats['loads'], PHP_EOL;
echo "errors: ", $stats['errors'], PHP_EOL;
echo "entries: ", $stats['entries'], PHP_EOL;
?>
===DONE===
--EXPECT--
Hello, world!
hits: 1
misses: 1
loads: 1
errors: 0
entries: 1
===DONE===
