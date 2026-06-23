--TEST--
ResourceCache: default INI settings enable cache with default weight limit
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$resource = "msg = Default cache settings\n";
FluentPhp\ResourceCache::fromString($resource);
FluentPhp\ResourceCache::fromString($resource);

$stats = FluentPhp\ResourceCache::getStats();
echo "max_weight: ", $stats['max_weight'], PHP_EOL;
echo "hits: ", $stats['hits'], PHP_EOL;
echo "misses: ", $stats['misses'], PHP_EOL;
?>
===DONE===
--EXPECT--
max_weight: 16777216
hits: 1
misses: 1
===DONE===
