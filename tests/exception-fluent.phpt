--TEST--
Exception is thrown for general Fluent errors
--FILE--
<?php

echo "--- 1: invalid language identifier ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('not-a-valid-lang!!!');
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}

echo "--- 2: not a ParserException ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('not-a-valid-lang!!!');
} catch (FluentPhp\ParserException $e) {
    echo "should not reach here\n";
} catch (FluentPhp\Exception $e) {
    echo "correctly not a ParserException\n";
}

echo "--- 3: missing message ID ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource("hello = Hello\n");
    $bundle->formatPattern('missing', []);
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}

// Attribute-only message loads and is found successfully, but formatPattern
// throws because there is no value to format.
echo "--- 4: attribute-only message has no value ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource("user =\n    .name = John\n");
    var_dump($bundle->hasMessage('user'));
    $bundle->formatPattern('user', []);
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}

echo "--- 5: unsupported argument type array ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('content = { $value }' . "\n");
    $bundle->formatPattern('content', ['value' => [1, 2, 3]]);
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}

echo "--- 6: unsupported argument type resource ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('content = { $value }' . "\n");
    $bundle->formatPattern('content', ['value' => tmpfile()]);
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}
?>
===DONE===
--EXPECT--
--- 1: invalid language identifier ---
FluentPhp\Exception
Invalid language identifier.
--- 2: not a ParserException ---
correctly not a ParserException
--- 3: missing message ID ---
FluentPhp\Exception
Message "missing" not found.
--- 4: attribute-only message has no value ---
bool(true)
FluentPhp\Exception
Message "user" has no value.
--- 5: unsupported argument type array ---
FluentPhp\Exception
Unsupported type for argument "value": Array.
--- 6: unsupported argument type resource ---
FluentPhp\Exception
Unsupported type for argument "value": Resource.
===DONE===
