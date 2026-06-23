--TEST--
ResourceCache: clear() resets all entries and stats
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$resource = <<<'FTL'
    hello = Hello!
    FTL;

FluentPhp\ResourceCache::fromString($resource);

$stats = FluentPhp\ResourceCache::getStats();
echo "before clear - misses: ", $stats['misses'], PHP_EOL;
echo "before clear - entries: ", $stats['entries'], PHP_EOL;

FluentPhp\ResourceCache::clear();

$stats = FluentPhp\ResourceCache::getStats();
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
