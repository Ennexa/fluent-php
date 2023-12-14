--TEST--
Test variables
--FILE--
<?php

class StringableObject {
	public function __toString() {
		return "__toString Value";
	}
}
$resource = <<<'FTL'
    content = { $value }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);

    $response = $bundle->formatPattern('content', ['value' => 1]);
    echo $response, PHP_EOL;

    $response = $bundle->formatPattern('content', ['value' => 2.1]);
    echo $response, PHP_EOL;

    $response = $bundle->formatPattern('content', ['value' => true]);
    echo $response, PHP_EOL;

    $response = $bundle->formatPattern('content', ['value' => false]);
    echo $response, PHP_EOL;

    $response = $bundle->formatPattern('content', ['value' => "Hello World"]);
    echo $response, PHP_EOL;

    $response = $bundle->formatPattern('content', ['value' => new StringableObject]);
    echo $response, PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}
?>
===DONE===
--EXPECT--
1
2.1
true
false
Hello World
__toString Value
===DONE===
