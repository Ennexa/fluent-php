---
title: Guide
nav_order: 2
permalink: /guide/
---

# Guide
{: .no_toc }

Use cases and worked examples for FluentPHP.
{: .fs-6 .fw-300 }

<details open markdown="block">
  <summary>Table of contents</summary>
  {: .text-delta }
- TOC
{:toc}
</details>

---

## Use cases

Common scenarios where teams reach for FluentPHP:

| Use case | Example scenario |
|:---------|:-----------------|
| **Localized web app UI** | Serve a web application in each user's language — pick the locale bundle per request and format page copy, labels, and notices with live data. |
| **Transactional emails & notifications** | Produce localized emails, SMS, and push messages — order confirmations, alerts, receipts — with names, amounts, and dates filled in. |
| **Multi-tenant SaaS** | Share one set of translation files across tenants and render every request in the tenant's own locale. |
| **Localized API responses** | Return messages and error text in the caller's language, chosen from an `Accept-Language` header or account preference. |
| **CLI tools, invoices & reports** | Localize command-line output and generated documents such as invoices, statements, and PDFs. |
| **Count- and gender-aware copy** | Cart counts, unread badges, gendered phrasing — let translators handle the variants in FTL instead of branching in PHP. |

The features that power these scenarios — variables, selectors, custom
functions, and error handling — are shown in the examples below.

## Variables

Inject request data into localized messages. Placeables (`{ $name }`) are
replaced with the matching argument passed to `formatPattern()`.

```php
$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource("welcome = Welcome, { $user }\n");

echo $bundle->formatPattern('welcome', [
    'user' => 'Asha',
]);
// Welcome, Asha
```

## Selectors

Selectors keep copy rules beside the translated text instead of branching in
controllers. Translators can adjust wording without PHP code changes.

```php
$bundle->addResource(<<<'FTL'
status = { $state ->
    [active] Account active
    [blocked] Account blocked
   *[other] Status unknown
}
FTL);

echo $bundle->formatPattern('status', ['state' => 'active']);
// Account active
```

The variant marked with `*` is the default, used when no other branch matches.

## PHP functions

Register PHP callables as Fluent functions so messages can use domain-specific
formatting without forcing that logic into translation files.

```php
$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource(<<<'FTL'
today = Today is { FORMAT_DATE($date) }
FTL);

$bundle->addFunction('FORMAT_DATE', function (DateTimeInterface $date): string {
    return $date->format('Y-m-d');
});

echo $bundle->formatPattern('today', ['date' => new DateTimeImmutable()]);
// Today is 2026-06-23
```

## Reusing resources across bundles

`FluentResource` is a parsed FTL resource. Parse once, then add the same
resource to multiple bundles.

```php
$resource = FluentPhp\FluentResource::fromFile(__DIR__ . '/messages.ftl');

$en = new FluentPhp\FluentBundle('en');
$en->addResource($resource);

$enGb = new FluentPhp\FluentBundle('en-GB');
$enGb->addResource($resource);
```

In a long-running PHP runtime (PHP-FPM, Swoole, RoadRunner, FrankenPHP), use
[`ResourceCache`]({{ '/cache/' | relative_url }}) instead so the parse is reused
across requests, not just across bundles within one request.

## Values

Message parameters may be strings, integers, floats, booleans, `null`, or
objects.

- **Stringable objects** are formatted through `__toString()`.
- **Non-stringable objects** format as `[Object]`.
- **Unsupported values** — such as arrays and resources — raise
  `FluentPhp\Exception`.

```php
$bundle->addResource("label = { $value }\n");

echo $bundle->formatPattern('label', ['value' => 42]);   // 42
echo $bundle->formatPattern('label', ['value' => true]); // true
```

Objects that implement `Stringable` are formatted with their `__toString()`:

```php
class Money implements Stringable
{
    public function __construct(private int $cents) {}

    public function __toString(): string
    {
        return number_format($this->cents / 100, 2);
    }
}

$bundle->addResource("price = Total: { $amount }\n");

echo $bundle->formatPattern('price', ['amount' => new Money(1299)]);
// Total: 12.99
```

## Error handling

All extension-specific exceptions extend `FluentPhp\Exception`, so a single
`catch` can cover everything, or you can target specific failures.

```php
use FluentPhp\FluentBundle;
use FluentPhp\ParserException;
use FluentPhp\ResolverException;

try {
    $bundle = new FluentBundle('en');
    $bundle->addResource($maybeInvalidFtl);
    echo $bundle->formatPattern('greeting', ['name' => 'Sam']);
} catch (ParserException $e) {
    // Invalid FTL syntax — structured details available:
    foreach ($e->getErrors() as $err) {
        fprintf(STDERR, "line %d, col %d: %s\n", $err['line'], $err['col'], $err['source']);
    }
} catch (ResolverException $e) {
    // Missing variables, unknown functions, etc.
    foreach ($e->getErrors() as $message) {
        fprintf(STDERR, "%s\n", $message);
    }
}
```

See the [API Reference]({{ '/api-reference/' | relative_url }}) for the full
exception hierarchy and the shape of `getErrors()`.

## Runnable examples

The [`example/`](https://github.com/Ennexa/fluent-php/tree/master/example)
directory contains runnable scripts:

- `hello-world.php`
- `has-message.php`
- `function.php`
- `selectors.php`
- `advanced.php`

Run one with the built extension:

```sh
php -d extension=target/debug/libfluent.so example/selectors.php
```
