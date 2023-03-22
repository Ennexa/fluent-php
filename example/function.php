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

    $datetime = new \DateTimeImmutable();

    $response = $bundle->formatPattern('log-time', ['date' => new \DateTimeImmutable('now')]);
    echo $response, PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}
