--TEST--
ParserException is thrown when FTL source has syntax errors
--FILE--
<?php

echo "--- 1: single parse error ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('= invalid-top-level');
} catch (FluentPhp\ParserException $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}

echo "--- 2: caught as Exception ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('= invalid-top-level');
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
}

echo "--- 3: multiple parse errors with getErrors() ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource("= error one\nvalid = OK\n= error two");
} catch (FluentPhp\ParserException $e) {
    echo $e->getMessage(), "\n";
    $errors = $e->getErrors();
    var_dump(count($errors));
    var_dump($errors[0]['line']);
    var_dump($errors[0]['col']);
    var_dump($errors[1]['line']);
    var_dump($errors[1]['col']);
}

echo "--- 4: parse error line/col with mixed LF and CRLF ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource("first = OK\r\nsecond = OK\n= error");
} catch (FluentPhp\ParserException $e) {
    $errors = $e->getErrors();
    var_dump($errors[0]['line']);
    var_dump($errors[0]['col']);
}
?>
===DONE===
--EXPECT--
--- 1: single parse error ---
FluentPhp\ParserException
Parse error: Line 1, col 0: Expected one of "a-zA-Z" - "= invalid-top-level"
--- 2: caught as Exception ---
FluentPhp\ParserException
--- 3: multiple parse errors with getErrors() ---
Parse errors:
  - Line 1, col 0: Expected one of "a-zA-Z" - "= error one"
  - Line 3, col 0: Expected one of "a-zA-Z" - "= error two"
int(2)
int(1)
int(0)
int(3)
int(0)
--- 4: parse error line/col with mixed LF and CRLF ---
int(3)
int(0)
===DONE===
