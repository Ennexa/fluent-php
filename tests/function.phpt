--TEST--
Test FluentBundle::addFunction
--FILE--
<?php

echo "--- 1: date formatting via registered function ---\n";
$resource = <<<'FTL'
    log-time = Entry time: { PHP_DATE_FORMAT($date, "Y-m-d G:i:a") }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);
    $bundle->addFunction("PHP_DATE_FORMAT", function (\DateTimeImmutable $date, string $format) {
        return $date->format($format);
    });
    $datetime = new \DateTimeImmutable('2023-03-22T15:22:00');
    echo $bundle->formatPattern('log-time', ['date' => $datetime]), PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}

echo "--- 2: function returning bool ---\n";
$resource = <<<'FTL'
    positive = { IS_POSITIVE($n) }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);
    $bundle->addFunction('IS_POSITIVE', fn($n) => $n > 0);
    echo $bundle->formatPattern('positive', ['n' => 5]), PHP_EOL;
    echo $bundle->formatPattern('positive', ['n' => -1]), PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}

// Variables preserve their PHP type when passed to functions, except integers
// which become floats (int -> FluentValue::Number -> float).
// Return values from one function are also correctly typed when passed to another.
echo "--- 3: variable types passed to functions ---\n";
$resource = <<<'FTL'
    var-type = { GET_TYPE($val) }
    ret-type = { GET_TYPE(IDENTITY($val)) }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);
    $bundle->addFunction('GET_TYPE', fn($val) => gettype($val));
    $bundle->addFunction('IDENTITY', fn($val) => $val);

    // Variable input types received by function
    echo $bundle->formatPattern('var-type', ['val' => 'hello']), PHP_EOL;       // string
    echo $bundle->formatPattern('var-type', ['val' => 42]), PHP_EOL;            // int -> double
    echo $bundle->formatPattern('var-type', ['val' => 3.14]), PHP_EOL;          // double
    echo $bundle->formatPattern('var-type', ['val' => true]), PHP_EOL;          // boolean
    echo $bundle->formatPattern('var-type', ['val' => new stdClass()]), PHP_EOL;// object

    // Return types passed through IDENTITY to GET_TYPE
    echo $bundle->formatPattern('ret-type', ['val' => 'hello']), PHP_EOL;       // string
    echo $bundle->formatPattern('ret-type', ['val' => 42]), PHP_EOL;            // int -> double -> double
    echo $bundle->formatPattern('ret-type', ['val' => 3.14]), PHP_EOL;          // double
    echo $bundle->formatPattern('ret-type', ['val' => true]), PHP_EOL;          // boolean
    echo $bundle->formatPattern('ret-type', ['val' => new stdClass()]), PHP_EOL;// object
} catch (\Exception $e) {
    print_r($e);
}
?>
===DONE===
--EXPECT--
--- 1: date formatting via registered function ---
Entry time: 2023-03-22 15:22:pm
--- 2: function returning bool ---
true
false
--- 3: variable types passed to functions ---
string
double
double
boolean
object
string
double
double
boolean
object
===DONE===
