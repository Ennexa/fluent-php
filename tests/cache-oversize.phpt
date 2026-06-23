--TEST--
ResourceCache: oversized entry is returned but not cached
--INI--
fluent.cache_max_entry_size=10
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$resource = <<<'FTL'
    greeting = Hello, this is a long enough message to exceed 10 bytes!
    FTL;

$r = FluentPhp\ResourceCache::fromString($resource);

$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource($r);
echo $bundle->formatPattern('greeting', []), PHP_EOL;

$stats = FluentPhp\ResourceCache::getStats();
echo "entries: ", $stats['entries'], PHP_EOL;
echo "loads: ", $stats['loads'], PHP_EOL;
echo "skipped_oversize: ", $stats['skipped_oversize'], PHP_EOL;
?>
===DONE===
--EXPECT--
Hello, this is a long enough message to exceed 10 bytes!
entries: 0
loads: 1
skipped_oversize: 1
===DONE===
