# FluentPHP

PHP bindings for the [Fluent](https://projectfluent.org/) localization system,
backed by the Rust [`fluent`](https://crates.io/crates/fluent) implementation.
The package and extension are named `fluent`/`fluent-php`.

The extension exposes a small PHP API for parsing Fluent Translation List
(`.ftl`) resources, adding them to locale bundles, formatting messages, and
optionally reusing parsed resources through an in-memory cache.

## Status

The public API is small, stable, and covered by PHPT tests, and is used in
production. Packaging and installation are still manual.

## Requirements

- PHP 8.0 or newer
- Rust 1.85 or newer
- A C/C++ build toolchain
- `libclang` headers for `ext-php-rs`

On Debian/Ubuntu-like systems:

```sh
apt-get update
apt-get install -y build-essential libclang-dev php-cli
```

## Build

Build a debug extension:

```sh
cargo build
```

Build an optimized extension:

```sh
cargo build --release
```

The extension artifact is platform-specific:

- Linux: `target/debug/libfluent.so` or `target/release/libfluent.so`
- macOS: `target/debug/libfluent.dylib` or `target/release/libfluent.dylib`
- Windows: `target\debug\fluent.dll` or `target\release\fluent.dll`

Load it explicitly when running PHP:

```sh
php -d extension=target/debug/libfluent.so example/hello-world.php
```

On macOS, use `libfluent.dylib` instead:

```sh
php -d extension=target/debug/libfluent.dylib example/hello-world.php
```

For a permanent installation, copy the built library into PHP's extension
directory and add an `extension=...` entry to your PHP ini configuration.

## Quick Start

```php
<?php

$resource = <<<'FTL'
hello = Hello, { $name }!
FTL;

$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource($resource);

echo $bundle->formatPattern('hello', ['name' => 'John']), PHP_EOL;
```

Output:

```text
Hello, John!
```

## API Overview

### `FluentPhp\FluentBundle`

`FluentBundle` owns messages for one locale.

```php
$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource("hello = Hello\n");

echo $bundle->hasMessage('hello') ? 'yes' : 'no';
echo $bundle->formatPattern('hello', []);
```

`addResource()` accepts either a raw FTL string or a `FluentResource` object.
String resources are parsed immediately and are not cached.

### `FluentPhp\FluentResource`

`FluentResource` is a parsed FTL resource. Use it when you want to parse once
and add the same resource to multiple bundles.

```php
$resource = FluentPhp\FluentResource::fromFile(__DIR__ . '/messages.ftl');

$en = new FluentPhp\FluentBundle('en');
$en->addResource($resource);
```

`FluentResource::fromString()` and `FluentResource::fromFile()` do not use the
process cache.

### `FluentPhp\ResourceCache`

`ResourceCache` keeps parsed `FluentResource` objects in memory, within the
current PHP process. This is useful for long-running PHP runtimes such as
PHP-FPM, Swoole, RoadRunner, and FrankenPHP.

```php
$resource = FluentPhp\ResourceCache::fromFile(__DIR__ . '/messages.ftl');

$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource($resource);
```

Each worker process has its own cache. Cache entries are not shared between
workers. `clear()` and `invalidateFile()` affect only the worker process that
handles that call; they do not clear caches in the rest of a PHP-FPM/Swoole/
RoadRunner/FrankenPHP worker pool.

Useful cache methods:

```php
FluentPhp\ResourceCache::fromString($source);
FluentPhp\ResourceCache::fromFile($path);
FluentPhp\ResourceCache::invalidateFile($path);
FluentPhp\ResourceCache::clear();
$stats = FluentPhp\ResourceCache::getStats();
```

`invalidateFile()` removes the cached file entry for a path. If the file has
already been deleted, invalidation is best-effort: it canonicalizes the nearest
existing parent directory so common symlinked paths such as `/tmp` on macOS
still work.

`invalidateFile()` and `clear()` affect only the worker process that runs them;
they do not clear other PHP-FPM, Swoole, or RoadRunner workers. In the `metadata`
and `checksum` modes this rarely matters: each worker detects file changes on its
own, so a deploy is picked up across the pool without any invalidation call.

## Cache Configuration

The cache can be configured with PHP ini settings:

```ini
fluent.cache_enabled=1
fluent.cache_max_weight=16M
fluent.cache_max_entry_size=2M
fluent.cache_file_validation=metadata
```

Settings:

- `fluent.cache_enabled`: enables or disables `ResourceCache`; default `1`.
- `fluent.cache_max_weight`: approximate total cache weight; default `16M`.
- `fluent.cache_max_entry_size`: approximate maximum single-entry weight;
  default `2M`.
- `fluent.cache_file_validation`: `metadata` or `checksum`; default `metadata`.

File validation modes:

- `metadata`: reuse a cached file resource when canonical path, modification
  time, and size match. The faster mode, but can serve stale content if a file
  changes while preserving modification time and size.
- `checksum`: read and hash the file before reusing a cached parse. This costs
  more I/O but detects same-size content changes.

## Custom Functions

You can register PHP callables as Fluent functions:

```php
<?php

$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource(<<<'FTL'
today = Today is { FORMAT_DATE($date) }
FTL);

$bundle->addFunction('FORMAT_DATE', function (DateTimeInterface $date): string {
    return $date->format('Y-m-d');
});

echo $bundle->formatPattern('today', ['date' => new DateTimeImmutable()]);
```

## Values

Message parameters may be strings, integers, floats, booleans, `null`, or
objects. Stringable objects are formatted through `__toString()`. Non-stringable
objects format as `[Object]`.

Unsupported values, such as arrays and resources, raise `FluentPhp\Exception`.

## Exceptions

All extension-specific exceptions extend `FluentPhp\Exception`.

- `FluentPhp\ParserException`: invalid FTL syntax. `getErrors()` returns line,
  column, and source snippets.
- `FluentPhp\ResolverException`: formatting failed because a message references
  missing variables, unknown functions, or other resolver errors. `getErrors()`
  returns resolver error messages.
- `FluentPhp\CacheException`: the process cache is unavailable.

## Tests

Run the Rust library build checks:

```sh
cargo test --lib
cargo clippy --lib -- -D warnings
```

Run PHPT tests against a built extension:

```sh
cargo build
php run-tests.php -n -d extension=target/debug/libfluent.so tests
```

On macOS:

```sh
php run-tests.php -n -d extension=target/debug/libfluent.dylib tests
```

## Examples

The [`example/`](example/) directory contains runnable PHP examples:

- `hello-world.php`
- `has-message.php`
- `function.php`
- `selectors.php`
- `advanced.php`

## Project Page

The GitHub Pages site is a Jekyll site (using the
[just-the-docs](https://just-the-docs.com/) theme) under [`docs/`](docs/):

- `index.md` — overview and quick start
- `guide.md` — use cases and examples
- `api-reference.md` — full API
- `cache.md` — process cache and ini settings

It is built and deployed by the
[`pages.yml`](.github/workflows/pages.yml) workflow. To enable publishing, set
**Settings → Pages → Build and deployment → Source** to **GitHub Actions**.

To preview locally:

```sh
cd docs
bundle install
bundle exec jekyll serve --baseurl ''
```

The `--baseurl ''` override is needed for local preview: `_config.yml` sets a
`baseurl` for the deployed project page, and without overriding it the theme's
CSS/JS resolve under that prefix and won't load at `http://localhost:4000/`.

The previous hand-written HTML landing page is kept (unbuilt) at
`docs/_legacy/index.html`.

## License

GPL-3.0. See [LICENSE](LICENSE).
