--TEST--
ResourceCache: default INI settings enable cache with default weight limit
--FILE--
<?php
FluentPHP\ResourceCache::clear();

$resource = "msg = Default cache settings\n";
FluentPHP\ResourceCache::fromString($resource);
FluentPHP\ResourceCache::fromString($resource);

$stats = FluentPHP\ResourceCache::getStats();
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
