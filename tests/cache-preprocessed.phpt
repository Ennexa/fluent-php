--TEST--
ResourceCache: preprocessed/transformed FTL strings are cached by content
--FILE--
<?php
FluentPhp\ResourceCache::clear();

// Simulate a preprocessor that generates different FTL from templates
$source1 = "greeting = Hello, {" . '$user' . "}!\n";
$source2 = "greeting = Hello, {" . '$user' . "}!\n";
$source3 = "greeting = Hello, {" . '$admin' . "}!\n";

// Same content should hit the cache
$r1 = FluentPhp\ResourceCache::fromString($source1);
$r2 = FluentPhp\ResourceCache::fromString($source2);

$stats = FluentPhp\ResourceCache::getStats();
echo "same content - hits: ", $stats['hits'], PHP_EOL;
echo "same content - misses: ", $stats['misses'], PHP_EOL;

// Different content should be a separate entry
$r3 = FluentPhp\ResourceCache::fromString($source3);

$stats = FluentPhp\ResourceCache::getStats();
echo "different content - entries: ", $stats['entries'], PHP_EOL;
echo "different content - misses: ", $stats['misses'], PHP_EOL;

// Verify both work correctly
$bundle1 = new FluentPhp\FluentBundle('en');
$bundle1->addResource($r1);
echo $bundle1->formatPattern('greeting', ['user' => 'Alice']), PHP_EOL;

$bundle2 = new FluentPhp\FluentBundle('en');
$bundle2->addResource($r3);
echo $bundle2->formatPattern('greeting', ['admin' => 'Bob']), PHP_EOL;
?>
===DONE===
--EXPECT--
same content - hits: 1
same content - misses: 1
different content - entries: 2
different content - misses: 2
Hello, Alice!
Hello, Bob!
===DONE===
