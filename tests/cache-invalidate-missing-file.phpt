--TEST--
ResourceCache: invalidateFile() can remove a cached file after it is deleted
--FILE--
<?php
FluentPHP\ResourceCache::clear();

$tmpFile = tempnam(sys_get_temp_dir(), 'fluent_test_');
file_put_contents($tmpFile, "msg = Cached before delete\n");

FluentPHP\ResourceCache::fromFile($tmpFile);
unlink($tmpFile);

echo "first invalidate: ", (FluentPHP\ResourceCache::invalidateFile($tmpFile) ? 'true' : 'false'), PHP_EOL;
echo "second invalidate: ", (FluentPHP\ResourceCache::invalidateFile($tmpFile) ? 'true' : 'false'), PHP_EOL;
?>
===DONE===
--EXPECT--
first invalidate: true
second invalidate: false
===DONE===
