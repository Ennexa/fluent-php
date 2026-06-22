--TEST--
ResourceCache: checksum validation detects same-size content changes
--INI--
fluent.cache_file_validation=checksum
--FILE--
<?php
FluentPHP\ResourceCache::clear();

$tmpFile = tempnam(sys_get_temp_dir(), 'fluent_test_');
file_put_contents($tmpFile, "msg = original\n");
$mtime = 1000000000;
touch($tmpFile, $mtime);
clearstatcache(true, $tmpFile);

FluentPHP\ResourceCache::fromFile($tmpFile);

file_put_contents($tmpFile, "msg = modified\n"); // same length
touch($tmpFile, $mtime);
clearstatcache(true, $tmpFile);

$r = FluentPHP\ResourceCache::fromFile($tmpFile);
$b = new FluentPHP\FluentBundle('en');
$b->addResource($r);
echo $b->formatPattern('msg', []), PHP_EOL;

$stats = FluentPHP\ResourceCache::getStats();
echo "content_hits: ", $stats['content_hits'], PHP_EOL;
echo "misses: ", $stats['misses'], PHP_EOL;
echo "loads: ", $stats['loads'], PHP_EOL;

unlink($tmpFile);
?>
===DONE===
--EXPECT--
modified
content_hits: 0
misses: 2
loads: 2
===DONE===
