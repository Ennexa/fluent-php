--TEST--
FluentBundle::addResource() accepts both string and FluentResource
--FILE--
<?php
// Test with string (backward compat)
$bundle1 = new FluentPHP\FluentBundle('en');
$bundle1->addResource("from-string = Works with string\n");
echo $bundle1->formatPattern('from-string', []), PHP_EOL;

// Test with FluentResource object (uncached)
$res = FluentPHP\FluentResource::fromString("from-resource = Works with FluentResource\n");
$bundle2 = new FluentPHP\FluentBundle('en');
$bundle2->addResource($res);
echo $bundle2->formatPattern('from-resource', []), PHP_EOL;

// Test with cached FluentResource
FluentPHP\ResourceCache::clear();
$cached = FluentPHP\ResourceCache::fromString("from-cache = Works with cached resource\n");
$bundle3 = new FluentPHP\FluentBundle('en');
$bundle3->addResource($cached);
echo $bundle3->formatPattern('from-cache', []), PHP_EOL;
?>
===DONE===
--EXPECT--
Works with string
Works with FluentResource
Works with cached resource
===DONE===
