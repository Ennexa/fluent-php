--TEST--
ResourceCache: eviction removes the least recently used entry
--INI--
fluent.cache_max_weight=150
--FILE--
<?php
FluentPHP\ResourceCache::clear();

$a = "a = Alpha value text\n";
$b = "b = Bravo value text\n";
$c = "c = Charlie value text\n";

FluentPHP\ResourceCache::fromString($a);
FluentPHP\ResourceCache::fromString($b);
FluentPHP\ResourceCache::fromString($a); // make A most recently used
FluentPHP\ResourceCache::fromString($c); // evicts B
FluentPHP\ResourceCache::fromString($a); // should still be cached
FluentPHP\ResourceCache::fromString($b); // reloads B

$stats = FluentPHP\ResourceCache::getStats();
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
