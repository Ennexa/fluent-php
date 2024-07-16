<?php

$resource = <<<'FTL'
    # Select expression only allows string/numeric types as selector.
    # We will use STR function to convert the boolean output of LT to string.
    log-time = User { $user->name} logged in at { $date->format("Y-m-d G:i:a") } {STR(LT($date, $deadline)) ->
            [true] before
            *[false] after
        } deadline.
    FTL;

/**
  * Pre-process the FTL to replace PHP style property access and
  * method call with Fluent functions.
  */
function preProcess(string $resource): string
{
    return preg_replace(
        [
            '~(\$[A-Za-z][A-Za-z0-9_]*)->([A-Za-z][A-Za-z0-9_]*)\(~',
            '~(\$[A-Za-z][A-Za-z0-9_]*)->([A-Za-z][A-Za-z0-9_]*)~',
        ],
        ['OBJ_METHOD(\1, "\2", ', 'OBJ_PROP(\1, "\2")'],
        $resource
    );
}

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addFunction('EQ', fn ($val1, $val2) => $val1 == $val2);
    $bundle->addFunction('GT', fn ($val1, $val2) => $val1 > $val2);
    $bundle->addFunction('LT', fn ($val1, $val2) => $val1 < $val2);
    $bundle->addFunction('GTE', fn ($val1, $val2) => $val1 >= $val2);
    $bundle->addFunction('LTE', fn ($val1, $val2) => $val1 <= $val2);
    $bundle->addFunction('STR', fn ($val) => is_bool($val) ? ($val ? "true" : "false") : (string)$val);
    $bundle->addFunction('OBJ_PROP', fn ($obj, $prop) => $obj->{$prop});
    $bundle->addFunction('OBJ_METHOD', fn ($obj, $method, ...$args) => $obj->{$method}(...$args));

    $bundle->addResource(preProcess($resource));

    $response = $bundle->formatPattern('log-time', [
        'date' => new \DateTimeImmutable('now'),
        'deadline' => new \DateTimeImmutable('tomorrow'),
        'user' => (object)['name' => 'John Doe'],
    ]);
    // Output: User John Doe logged in at 2023-03-22 15:22:pm before deadline.
    echo $response, PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}
