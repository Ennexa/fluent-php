--TEST--
PHP variable types as Fluent pattern arguments
--FILE--
<?php

class StringableObject {
    public function __toString() {
        return "__toString Value";
    }
}

class NonStringableObject {}

$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource('content = { $value }' . "\n");

echo "--- 1: integer ---\n";
echo $bundle->formatPattern('content', ['value' => 1]), PHP_EOL;

echo "--- 2: float ---\n";
echo $bundle->formatPattern('content', ['value' => 2.1]), PHP_EOL;

echo "--- 3: boolean true ---\n";
echo $bundle->formatPattern('content', ['value' => true]), PHP_EOL;

echo "--- 4: boolean false ---\n";
echo $bundle->formatPattern('content', ['value' => false]), PHP_EOL;

echo "--- 5: string ---\n";
echo $bundle->formatPattern('content', ['value' => 'Hello World']), PHP_EOL;

echo "--- 6: Stringable object ---\n";
echo $bundle->formatPattern('content', ['value' => new StringableObject()]), PHP_EOL;

echo "--- 7: non-Stringable object ---\n";
echo $bundle->formatPattern('content', ['value' => new NonStringableObject()]), PHP_EOL;

echo "--- 8: null ---\n";
var_dump($bundle->formatPattern('content', ['value' => null]));
?>
===DONE===
--EXPECT--
--- 1: integer ---
1
--- 2: float ---
2.1
--- 3: boolean true ---
true
--- 4: boolean false ---
false
--- 5: string ---
Hello World
--- 6: Stringable object ---
__toString Value
--- 7: non-Stringable object ---
[Object]
--- 8: null ---
string(0) ""
===DONE===
