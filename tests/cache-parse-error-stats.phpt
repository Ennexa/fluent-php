--TEST--
ResourceCache: parse failures update error stats without caching entries
--FILE--
<?php
FluentPHP\ResourceCache::clear();

try {
    FluentPHP\ResourceCache::fromString("= invalid-top-level");
} catch (FluentPHP\ParserException $e) {
    echo get_class($e), PHP_EOL;
}

$stats = FluentPHP\ResourceCache::getStats();
echo "entries: ", $stats['entries'], PHP_EOL;
echo "misses: ", $stats['misses'], PHP_EOL;
echo "loads: ", $stats['loads'], PHP_EOL;
echo "errors: ", $stats['errors'], PHP_EOL;
?>
===DONE===
--EXPECT--
FluentPHP\ParserException
entries: 0
misses: 1
loads: 0
errors: 1
===DONE===
