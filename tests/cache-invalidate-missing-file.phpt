--TEST--
ResourceCache: invalidateFile() can remove a cached file after it is deleted
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$tmpFile = tempnam(sys_get_temp_dir(), 'fluent_test_');
file_put_contents($tmpFile, "msg = Cached before delete\n");

FluentPhp\ResourceCache::fromFile($tmpFile);
unlink($tmpFile);

echo "first invalidate: ", (FluentPhp\ResourceCache::invalidateFile($tmpFile) ? 'true' : 'false'), PHP_EOL;
echo "second invalidate: ", (FluentPhp\ResourceCache::invalidateFile($tmpFile) ? 'true' : 'false'), PHP_EOL;
?>
===DONE===
--EXPECT--
first invalidate: true
second invalidate: false
===DONE===
