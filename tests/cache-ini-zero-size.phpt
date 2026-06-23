--TEST--
ResourceCache: zero-sized cache INI values fall back to defaults
--INI--
fluent.cache_max_weight=0
fluent.cache_max_entry_size=0
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$resource = "msg = Zero size settings use defaults\n";
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
