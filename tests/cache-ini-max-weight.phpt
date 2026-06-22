--TEST--
ResourceCache: INI fluent.cache.max_weight sets max_weight
--INI--
fluent.cache_max_weight=32M
--FILE--
<?php
$stats = FluentPHP\ResourceCache::getStats();
echo "max_weight: ", $stats['max_weight'], PHP_EOL;
?>
===DONE===
--EXPECT--
max_weight: 33554432
===DONE===
