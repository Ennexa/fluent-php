--TEST--
ResourceCache: invalidateFile() forces re-parse on next load
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$tmpFile = tempnam(sys_get_temp_dir(), 'fluent_test_');
file_put_contents($tmpFile, "msg = Version 1\n");

$r1 = FluentPhp\ResourceCache::fromFile($tmpFile);
$bundle1 = new FluentPhp\FluentBundle('en');
$bundle1->addResource($r1);
echo $bundle1->formatPattern('msg', []), PHP_EOL;

$stats = FluentPhp\ResourceCache::getStats();
echo "misses after first load: ", $stats['misses'], PHP_EOL;

FluentPhp\ResourceCache::invalidateFile($tmpFile);

sleep(1);
file_put_contents($tmpFile, "msg = Version 2\n");

$r2 = FluentPhp\ResourceCache::fromFile($tmpFile);
$bundle2 = new FluentPhp\FluentBundle('en');
$bundle2->addResource($r2);
echo $bundle2->formatPattern('msg', []), PHP_EOL;

$stats = FluentPhp\ResourceCache::getStats();
echo "misses after invalidate+reload: ", $stats['misses'], PHP_EOL;

unlink($tmpFile);
?>
===DONE===
--EXPECT--
Version 1
misses after first load: 1
Version 2
misses after invalidate+reload: 2
===DONE===
