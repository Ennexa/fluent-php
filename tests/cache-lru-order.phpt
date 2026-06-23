--TEST--
ResourceCache: eviction removes the least recently used entry
--INI--
fluent.cache_max_weight=150
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$a = "a = Alpha value text\n";
$b = "b = Bravo value text\n";
$c = "c = Charlie value text\n";

FluentPhp\ResourceCache::fromString($a);
FluentPhp\ResourceCache::fromString($b);
FluentPhp\ResourceCache::fromString($a); // make A most recently used
FluentPhp\ResourceCache::fromString($c); // evicts B
FluentPhp\ResourceCache::fromString($a); // should still be cached
FluentPhp\ResourceCache::fromString($b); // reloads B

$stats = FluentPhp\ResourceCache::getStats();
echo "hits: ", $stats['hits'], PHP_EOL;
echo "misses: ", $stats['misses'], PHP_EOL;
echo "loads: ", $stats['loads'], PHP_EOL;
echo "evictions: ", $stats['evictions'], PHP_EOL;
?>
===DONE===
--EXPECT--
hits: 2
misses: 4
loads: 4
evictions: 2
===DONE===
