--TEST--
Exception is thrown on duplicate entries
--FILE--
<?php

echo "--- 1: duplicate message ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource("hello = First\n");
    $bundle->addResource("hello = Second\n");
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}

echo "--- 2: duplicate term ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource("-brand = Acme\n");
    $bundle->addResource("-brand = Corp\n");
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}

echo "--- 3: duplicate function ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addFunction('MY_FUNC', fn() => 'a');
    $bundle->addFunction('MY_FUNC', fn() => 'b');
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
}

echo "--- 4: multiple duplicate IDs ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource("hello = Hello\nworld = World\n");
    $bundle->addResource("hello = Hi\nworld = Earth\n");
} catch (FluentPhp\Exception $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
    var_dump(str_contains($e->getMessage(), '"hello"'));
    var_dump(str_contains($e->getMessage(), '"world"'));
}
?>
===DONE===
--EXPECT--
--- 1: duplicate message ---
FluentPHP\Exception
Attempt to override an existing message: "hello".
--- 2: duplicate term ---
FluentPHP\Exception
Attempt to override an existing term: "brand".
--- 3: duplicate function ---
FluentPHP\Exception
Attempt to override an existing function: "MY_FUNC".
--- 4: multiple duplicate IDs ---
FluentPHP\Exception
Attempt to override existing entries: "hello", "world".
bool(true)
bool(true)
===DONE===
