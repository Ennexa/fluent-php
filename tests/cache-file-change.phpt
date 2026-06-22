--TEST--
ResourceCache: modified file triggers reparse
--FILE--
<?php
FluentPHP\ResourceCache::clear();

$tmpFile = tempnam(sys_get_temp_dir(), 'fluent_test_');
file_put_contents($tmpFile, "msg = Original\n");

$r1 = FluentPHP\ResourceCache::fromFile($tmpFile);
$bundle1 = new FluentPHP\FluentBundle('en');
$bundle1->addResource($r1);
echo $bundle1->formatPattern('msg', []), PHP_EOL;

$stats = FluentPHP\ResourceCache::getStats();
echo "misses: ", $stats['misses'], PHP_EOL;
echo "loads: ", $stats['loads'], PHP_EOL;

// Modify the file (sleep to ensure mtime changes)
sleep(1);
file_put_contents($tmpFile, "msg = Modified\n");

$r2 = FluentPHP\ResourceCache::fromFile($tmpFile);
$bundle2 = new FluentPHP\FluentBundle('en');
$bundle2->addResource($r2);
echo $bundle2->formatPattern('msg', []), PHP_EOL;

$stats = FluentPHP\ResourceCache::getStats();
echo "misses after change: ", $stats['misses'], PHP_EOL;
echo "loads after change: ", $stats['loads'], PHP_EOL;

unlink($tmpFile);
?>
===DONE===
--EXPECT--
Original
misses: 1
loads: 1
Modified
misses after change: 2
loads after change: 2
===DONE===
