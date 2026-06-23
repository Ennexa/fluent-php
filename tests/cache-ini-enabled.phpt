--TEST--
ResourceCache: INI fluent.cache.enabled=0 disables caching
--INI--
fluent.cache_enabled=0
--FILE--
<?php
$resource = <<<'FTL'
    msg = Hello from INI test
    FTL;

$r = FluentPhp\ResourceCache::fromString($resource);

$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource($r);
echo $bundle->formatPattern('msg', []), PHP_EOL;

$stats = FluentPhp\ResourceCache::getStats();
echo "entries: ", $stats['entries'], PHP_EOL;
?>
===DONE===
--EXPECT--
Hello from INI test
entries: 0
===DONE===
