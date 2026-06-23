--TEST--
ResourceCache: invalidateFile() removes a deleted file loaded through a symlinked path
--SKIPIF--
<?php
if (PHP_OS_FAMILY === 'Windows' || !function_exists('symlink')) {
    die('skip symlink support required');
}
?>
--FILE--
<?php
FluentPhp\ResourceCache::clear();

$base = sys_get_temp_dir() . DIRECTORY_SEPARATOR . 'fluent_cache_' . getmypid() . '_' . bin2hex(random_bytes(4));
$realDir = $base . '_real';
$linkDir = $base . '_link';

mkdir($realDir);
symlink($realDir, $linkDir);

$path = $linkDir . DIRECTORY_SEPARATOR . 'messages.ftl';
file_put_contents($path, "msg = Cached through symlink\n");

FluentPhp\ResourceCache::fromFile($path);
unlink($path);

echo "first invalidate: ", (FluentPhp\ResourceCache::invalidateFile($path) ? 'true' : 'false'), PHP_EOL;
echo "entries: ", FluentPhp\ResourceCache::getStats()['entries'], PHP_EOL;

unlink($linkDir);
rmdir($realDir);
?>
===DONE===
--EXPECT--
first invalidate: true
entries: 0
===DONE===
