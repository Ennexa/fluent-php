--TEST--
Variant selectors for numeric, string, and boolean values
--FILE--
<?php

echo "--- 1: numeric plural selector ---\n";
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
    echo $bundle->formatPattern('emails', ['unreadEmails' => 1]), PHP_EOL;
    echo $bundle->formatPattern('emails', ['unreadEmails' => 2]), PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}

// String variables match against string variant keys exactly
echo "--- 2: string exact match selector ---\n";
$resource = <<<'FTL'
    status = { $status ->
        [active] Active
        [inactive] Inactive
       *[other] Unknown
    }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);
    echo $bundle->formatPattern('status', ['status' => 'active']), PHP_EOL;
    echo $bundle->formatPattern('status', ['status' => 'inactive']), PHP_EOL;
    echo $bundle->formatPattern('status', ['status' => 'unknown-value']), PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}

// Numbers match against numeric variant keys exactly; the *[other] branch also
// catches numbers with no exact match via plural category rules
echo "--- 3: numeric exact match selector ---\n";
$resource = <<<'FTL'
    rating = { $rating ->
        [1] Poor
        [2] Fair
        [3] Good
       *[other] Unknown
    }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);

    echo $bundle->formatPattern('rating', ['rating' => 1]), PHP_EOL;    // int matches [1]
    echo $bundle->formatPattern('rating', ['rating' => 2.0]), PHP_EOL;  // float matches [2]
    echo $bundle->formatPattern('rating', ['rating' => 1.5]), PHP_EOL;  // no exact match -> *[other]
} catch (\Exception $e) {
    print_r($e);
}

// Fluent's selector matching only handles String and Number variants.
// The idiomatic approach for other types is to convert to a string before
// passing or using a user-defined function in the FTL.
echo "--- 4: boolean falls to default without string conversion ---\n";
$resource = <<<'FTL'
    flag-direct = { $flag ->
        [true] Matched true
        [false] Matched false
       *[other] Default used
    }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);
    echo $bundle->formatPattern('flag-direct', ['flag' => true]), PHP_EOL;
    echo $bundle->formatPattern('flag-direct', ['flag' => false]), PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}

echo "--- 5: boolean matches string key via STR conversion ---\n";
$resource = <<<'FTL'
    flag-str = { STR($flag) ->
        [true] Matched true
        [false] Matched false
       *[other] Default used
    }
    FTL;

try {
    $bundle = new FluentPhp\FluentBundle('en');
    $bundle->addResource($resource);
    $bundle->addFunction('STR', fn($val) => is_bool($val) ? ($val ? 'true' : 'false') : (string)$val);
    echo $bundle->formatPattern('flag-str', ['flag' => true]), PHP_EOL;
    echo $bundle->formatPattern('flag-str', ['flag' => false]), PHP_EOL;
} catch (\Exception $e) {
    print_r($e);
}
?>
===DONE===
--EXPECT--
--- 1: numeric plural selector ---
You have one unread email.
You have 2 unread emails.
--- 2: string exact match selector ---
Active
Inactive
Unknown
--- 3: numeric exact match selector ---
Poor
Fair
Unknown
--- 4: boolean falls to default without string conversion ---
Default used
Default used
--- 5: boolean matches string key via STR conversion ---
Matched true
Matched false
===DONE===
