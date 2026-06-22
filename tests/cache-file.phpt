--TEST--
ResourceCache: file-based caching with fromFile
--FILE--
<?php
FluentPHP\ResourceCache::clear();

$tmpFile = tempnam(sys_get_temp_dir(), 'fluent_test_');
file_put_contents($tmpFile, "greeting = Hello from file!\n");

$r1 = FluentPHP\ResourceCache::fromFile($tmpFile);
$r2 = FluentPHP\ResourceCache::fromFile($tmpFile);

$bundle = new FluentPHP\FluentBundle('en');
$bundle->addResource($r1);
echo $bundle->formatPattern('greeting', []), PHP_EOL;

$stats = FluentPHP\ResourceCache::getStats();
echo "hits: ", $stats['hits'], PHP_EOL;
echo "metadata_hits: ", $stats['metadata_hits'], PHP_EOL;
echo "misses: ", $stats['misses'], PHP_EOL;
echo "loads: ", $stats['loads'], PHP_EOL;
echo "errors: ", $stats['errors'], PHP_EOL;

unlink($tmpFile);
?>
===DONE===
--EXPECT--
Hello from file!
hits: 1
metadata_hits: 1
misses: 1
loads: 1
errors: 0
===DONE===
