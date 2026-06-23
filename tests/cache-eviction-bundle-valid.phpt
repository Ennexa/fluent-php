--TEST--
ResourceCache: evicted entry does not break existing bundle
--INI--
fluent.cache_max_weight=200
--FILE--
<?php
FluentPhp\ResourceCache::clear();

// max_weight=200 so individual entries fit but not all together.
$r1 = FluentPhp\ResourceCache::fromString("msg1 = First message that is long enough to have weight\n");
$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource($r1);

// This entry should evict the first one.
FluentPhp\ResourceCache::fromString("msg2 = Second message that is also quite long to cause eviction\n");

$stats = FluentPhp\ResourceCache::getStats();
echo "evictions: ", ($stats['evictions'] > 0 ? 'yes' : 'no'), PHP_EOL;

// The bundle should still work even if the cache entry was evicted
echo $bundle->formatPattern('msg1', []), PHP_EOL;
?>
===DONE===
--EXPECT--
evictions: yes
First message that is long enough to have weight
===DONE===
