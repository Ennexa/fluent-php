---
title: API Reference
nav_order: 3
permalink: /api-reference/
---

# API Reference
{: .no_toc }

Every public class lives in the `FluentPhp` namespace.
{: .fs-6 .fw-300 }

<details open markdown="block">
  <summary>Table of contents</summary>
  {: .text-delta }
- TOC
{:toc}
</details>

---

## FluentPhp\FluentBundle

A bundle owns the messages for one locale. Add resources to it, optionally
register functions, then format messages by id.

### __construct

```php
public function __construct(string $langCode)
```

Create a bundle for a locale (a BCP-47 language identifier such as `en`,
`en-GB`, or `pt-BR`).

- **Throws** `FluentPhp\Exception` if the language identifier is invalid.

### addResource

```php
public function addResource(string|FluentResource $resource): void
```

Add a parsed resource to the bundle. Accepts either a
[`FluentResource`](#fluentphpfluentresource) object or a raw FTL string.

{: .note }
> String arguments are parsed **inline and are not cached**. To reuse a parse,
> pass a `FluentResource` from [`ResourceCache`](#fluentphpresourcecache) or
> `FluentResource::fromFile()`.

- **Throws** `FluentPhp\ParserException` if a string argument contains syntax errors.
- **Throws** `FluentPhp\Exception` if any entry in the resource duplicates an existing one.

### addFunction

```php
public function addFunction(string $name, callable $callable): void
```

Register a PHP callable as a Fluent function, callable from FTL as
`{ NAME($arg) }`.

- **Throws** `FluentPhp\Exception` if a function with that name is already registered.

### formatPattern

```php
public function formatPattern(string $messageId, array $parameters): string
```

Format a message by id, substituting `$parameters` into its placeables.
See [Values]({{ '/guide/#values' | relative_url }}) for accepted parameter types.

- **Throws** `FluentPhp\Exception` if the message is not found, has no value, or an argument type is unsupported.
- **Throws** `FluentPhp\ResolverException` if the pattern references undefined variables or functions.

### hasMessage

```php
public function hasMessage(string $messageId): bool
```

Return whether the bundle contains a message with the given id.

---

## FluentPhp\FluentResource

A parsed FTL resource that can be added to one or more bundles. Both
constructors **bypass the process cache** — use
[`ResourceCache`](#fluentphpresourcecache) when you want caching.

`final class`, not instantiable directly; use the static factories.

### fromString

```php
public static function fromString(string $source): self
```

Parse an FTL source string without using the process cache.

- **Throws** `FluentPhp\ParserException` if the FTL source contains syntax errors.

### fromFile

```php
public static function fromFile(string $path): self
```

Read and parse an FTL file without using the process cache.

- **Throws** `FluentPhp\ParserException` if the FTL file contains syntax errors.
- **Throws** `FluentPhp\Exception` if the file cannot be read.

---

## FluentPhp\ResourceCache

A cache of parsed `FluentResource` objects, reused across requests. All methods
are static; the class is not meant to be instantiated.

The cache is process-local. In multi-worker runtimes, each worker has its own
cache, and `clear()` / `invalidateFile()` affect only the worker process that
handles that call. They do not broadcast to the rest of a PHP-FPM, Swoole,
RoadRunner, or FrankenPHP worker pool.

For how caching works, configuration, validation modes, and statistics, see the
[Cache page]({{ '/cache/' | relative_url }}).

### fromString

```php
public static function fromString(string $source): FluentResource
```

Return a parsed resource cached by source-content identity (a 128-bit content
hash).

- **Throws** `FluentPhp\ParserException` if the FTL source contains syntax errors.
- **Throws** `FluentPhp\CacheException` if the cache is unavailable.

### fromFile

```php
public static function fromFile(string $path): FluentResource
```

Return a parsed resource cached by canonical file path. By default, file changes
are detected using path, size, and modification time; set
`fluent.cache_file_validation=checksum` to hash the file before reusing a cached
parse.

- **Throws** `FluentPhp\ParserException` if the FTL file contains syntax errors.
- **Throws** `FluentPhp\Exception` if the file cannot be read.
- **Throws** `FluentPhp\CacheException` if the cache is unavailable.

### invalidateFile

```php
public static function invalidateFile(string $path): bool
```

Invalidate a cached file entry in the current worker process. The next
`fromFile()` call in that worker reloads the file. Existing `FluentResource`
objects and bundles remain valid. Returns whether an entry was removed.

{: .warning }
> In multi-worker runtimes this does not invalidate other workers. Use
> validating cache modes (`metadata`/`checksum`) or reload/restart the worker
> pool when every worker must see new translation files.

### clear

```php
public static function clear(): void
```

Remove all entries from the current worker process cache. Existing
`FluentResource` objects and bundles remain valid.

{: .warning }
> In multi-worker runtimes this clears only the worker handling the call.

- **Throws** `FluentPhp\CacheException` if the cache is unavailable.

### getStats

```php
public static function getStats(): array
```

Return cache statistics for the current process:

| Key | Meaning |
|:----|:--------|
| `entries` | Number of cached resources (string + file). |
| `cache_weight` | Approximate total weight currently held. |
| `hits` | Total cache hits. |
| `metadata_hits` | File hits validated by path, modification time, and size. |
| `content_hits` | File hits validated by content hash. |
| `misses` | Lookups that required a parse. |
| `loads` | Resources parsed and inserted. |
| `errors` | Parse or I/O errors recorded. |
| `evictions` | Entries removed by LRU eviction. |
| `skipped_oversize` | Parses returned but not cached (over `max_entry_size`). |
| `max_weight` | Configured weight cap. |
| `pid` | Process id that owns this cache. |

- **Throws** `FluentPhp\CacheException` if the cache is unavailable.

---

## Exceptions

All extension-specific exceptions extend `FluentPhp\Exception`, which extends
PHP's `\Exception`.

### FluentPhp\Exception

Base class for every error this extension raises. Catch it to handle any
FluentPHP failure.

### FluentPhp\ParserException

Invalid FTL syntax.

```php
/** @return array<array{line: int, col: int, source: string}> */
public function getErrors(): array
```

`getErrors()` returns one entry per syntax error, each with the line, column,
and a source snippet.

### FluentPhp\ResolverException

Formatting failed because a message references missing variables, unknown
functions, or other resolver errors.

```php
/** @return array<string> */
public function getErrors(): array
```

`getErrors()` returns the resolver error messages.

### FluentPhp\CacheException

The process cache is unavailable (for example, an internal lock was poisoned).
