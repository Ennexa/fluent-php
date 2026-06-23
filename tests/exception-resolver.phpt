--TEST--
ResolverException is thrown when pattern resolution fails
--FILE--
<?php

echo "--- 1: missing variable ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('content = { $value }' . "\n");
    $bundle->formatPattern('content', []);
} catch (FluentPhp\ResolverException $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
    $errors = $e->getErrors();
    var_dump(count($errors));
    var_dump($errors[0]);
}

echo "--- 2: unknown function ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('content = { UNKNOWN_FUNC() }' . "\n");
    $bundle->formatPattern('content', []);
} catch (FluentPhp\ResolverException $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
    $errors = $e->getErrors();
    var_dump(count($errors));
    var_dump($errors[0]);
}

echo "--- 3: multiple resolver errors ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('content = { $missing } { UNKNOWN_FUNC() }' . "\n");
    $bundle->formatPattern('content', []);
} catch (FluentPhp\ResolverException $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
    $errors = $e->getErrors();
    var_dump(count($errors));
    var_dump($errors[0]);
    var_dump($errors[1]);
}

echo "--- 4: getMessage() truncates beyond 3 errors ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('content = { $a } { $b } { $c } { $d }' . "\n");
    $bundle->formatPattern('content', []);
} catch (FluentPhp\ResolverException $e) {
    echo get_class($e), "\n";
    echo $e->getMessage(), "\n";
    var_dump(count($e->getErrors()));
}

echo "--- 5: caught as Exception ---\n";
try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource('content = { $value }' . "\n");
    $bundle->formatPattern('content', []);
} catch (FluentPhp\Exception $e) {
    echo "caught\n";
}
?>
===DONE===
--EXPECT--
--- 1: missing variable ---
FluentPhp\ResolverException
Resolution failed with error: Unknown variable: $value
int(1)
string(24) "Unknown variable: $value"
--- 2: unknown function ---
FluentPhp\ResolverException
Resolution failed with error: Unknown function: UNKNOWN_FUNC()
int(1)
string(32) "Unknown function: UNKNOWN_FUNC()"
--- 3: multiple resolver errors ---
FluentPhp\ResolverException
Resolution failed with 2 errors: Unknown variable: $missing; Unknown function: UNKNOWN_FUNC()
int(2)
string(26) "Unknown variable: $missing"
string(32) "Unknown function: UNKNOWN_FUNC()"
--- 4: getMessage() truncates beyond 3 errors ---
FluentPhp\ResolverException
Resolution failed with 4 errors: Unknown variable: $a; Unknown variable: $b; Unknown variable: $c; and 1 more
int(4)
--- 5: caught as Exception ---
caught
===DONE===
