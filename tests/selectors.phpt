--TEST--
Test selectors
--FILE--
<?php

$resource = <<<'FTL'
    emails =
        { $unreadEmails ->
            [one] You have one unread email.
           *[other] You have { $unreadEmails } unread emails.
        }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);

    $response = $bundle->formatPattern('emails', ['unreadEmails' => 1]);
    echo $response, PHP_EOL;

    $response = $bundle->formatPattern('emails', ['unreadEmails' => 2]);
    echo $response, PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}
?>
===DONE===
--EXPECT--
You have one unread email.
You have 2 unread emails.
===DONE===
