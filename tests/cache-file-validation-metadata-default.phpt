--TEST--
ResourceCache: metadata validation trusts stat results (default behavior)
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$tmpFile = tempnam(sys_get_temp_dir(), 'fluent_test_');
file_put_contents($tmpFile, "msg = original\n");
$mtime = 1000000000;
touch($tmpFile, $mtime);
clearstatcache(true, $tmpFile);

FluentPhp\ResourceCache::fromFile($tmpFile);

file_put_contents($tmpFile, "msg = modified\n"); // same length
touch($tmpFile, $mtime);
clearstatcache(true, $tmpFile);

$r = FluentPhp\ResourceCache::fromFile($tmpFile);
$b = new FluentPhp\FluentBundle('en');
$b->addResource($r);
echo $b->formatPattern('msg', []), PHP_EOL;

$stats = FluentPhp\ResourceCache::getStats();
echo "metadata_hits: ", $stats['metadata_hits'], PHP_EOL;

unlink($tmpFile);
?>
===DONE===
--EXPECT--
original
metadata_hits: 1
===DONE===
