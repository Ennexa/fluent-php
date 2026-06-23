--TEST--
ResourceCache: parse failures update error stats without caching entries
--FILE--
<?php
FluentPhp\ResourceCache::clear();

try {
    FluentPhp\ResourceCache::fromString("= invalid-top-level");
} catch (FluentPhp\ParserException $e) {
    echo get_class($e), PHP_EOL;
}

$stats = FluentPhp\ResourceCache::getStats();
echo "entries: ", $stats['entries'], PHP_EOL;
echo "misses: ", $stats['misses'], PHP_EOL;
echo "loads: ", $stats['loads'], PHP_EOL;
echo "errors: ", $stats['errors'], PHP_EOL;
?>
===DONE===
--EXPECT--
FluentPhp\ParserException
entries: 0
misses: 1
loads: 0
errors: 1
===DONE===
