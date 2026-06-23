--TEST--
FluentResource: uncached fromString() and fromFile() work correctly
--FILE--
<?php
// fromString
$r1 = FluentPhp\FluentResource::fromString("greeting = Hello from string!\n");
$bundle1 = new FluentPhp\FluentBundle('en');
$bundle1->addResource($r1);
echo $bundle1->formatPattern('greeting', []), PHP_EOL;

// fromFile
$tmpFile = tempnam(sys_get_temp_dir(), 'fluent_test_');
file_put_contents($tmpFile, "farewell = Goodbye from file!\n");

$r2 = FluentPhp\FluentResource::fromFile($tmpFile);
$bundle2 = new FluentPhp\FluentBundle('en');
$bundle2->addResource($r2);
echo $bundle2->formatPattern('farewell', []), PHP_EOL;

unlink($tmpFile);
?>
===DONE===
--EXPECT--
Hello from string!
Goodbye from file!
===DONE===
