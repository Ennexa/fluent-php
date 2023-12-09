--TEST--
Test FluentBundle::addFunction
--FILE--
<?php

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

    $response = $bundle->formatPattern('log-time', ['date' => $datetime]);
    echo $response, PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}
?>
===DONE===
--EXPECT--
Entry time: 2023-03-22 15:22:pm
===DONE===
