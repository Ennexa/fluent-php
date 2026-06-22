--TEST--
ResourceCache: clear() resets all entries and stats
--FILE--
<?php
FluentPHP\ResourceCache::clear();

$resource = <<<'FTL'
    hello = Hello!
    FTL;

FluentPHP\ResourceCache::fromString($resource);

$stats = FluentPHP\ResourceCache::getStats();
echo "before clear - misses: ", $stats['misses'], PHP_EOL;
echo "before clear - entries: ", $stats['entries'], PHP_EOL;

FluentPHP\ResourceCache::clear();

$stats = FluentPHP\ResourceCache::getStats();
echo "after clear - hits: ", $stats['hits'], PHP_EOL;
echo "after clear - misses: ", $stats['misses'], PHP_EOL;
echo "after clear - entries: ", $stats['entries'], PHP_EOL;
?>
===DONE===
--EXPECT--
before clear - misses: 1
before clear - entries: 1
after clear - hits: 0
after clear - misses: 0
after clear - entries: 0
===DONE===
