---
title: Home
nav_order: 1
description: "FluentPHP: PHP bindings for the Fluent localization system."
permalink: /
---

# FluentPHP
{: .fs-9 }

PHP bindings for the Fluent localization system. Format localized,
natural-sounding messages with variables, plurals, and other variants — and
extend them with your own PHP functions.
{: .fs-6 .fw-300 }

[Get started](#quick-start){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/Ennexa/fluent-php){: .btn .fs-5 .mb-4 .mb-md-0 }

---

FluentPHP brings [Project Fluent](https://projectfluent.org/) — Mozilla's
localization system — to PHP. Translators write messages in Fluent's `.ftl`
format; your application formats them by id with runtime data.

- **Focused PHP API** — bundles, resources, functions, and a clear exception hierarchy.
- **Built on Project Fluent** — Mozilla's reference implementation for parsing and resolution.
- **Resource caching** — optionally reuse parsed translations across requests, sized via `php.ini`.

## Quick start

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

## Where to next

| Page | What's there |
|:-----|:-------------|
| [Guide]({{ '/guide/' | relative_url }}) | Use cases and worked examples: variables, selectors, PHP functions, values, and error handling. |
| [API Reference]({{ '/api-reference/' | relative_url }}) | Every class and method: `FluentBundle`, `FluentResource`, `ResourceCache`, and the exception hierarchy. |
| [Cache]({{ '/cache/' | relative_url }}) | How resource caching works, all `php.ini` settings, file-validation modes, and statistics. |

## Requirements

- PHP 8.0 or newer
- Rust 1.80 or newer
- A C/C++ build toolchain
- `libclang` headers for `ext-php-rs`

On Debian/Ubuntu-like systems:

```sh
apt-get update
apt-get install -y build-essential libclang-dev php-cli
```

## Install

Build a debug or release extension with Cargo:

```sh
cargo build            # target/debug
cargo build --release  # target/release
```

The artifact is platform-specific:

- Linux: `target/debug/libfluent.so`
- macOS: `target/debug/libfluent.dylib`
- Windows: `target\debug\fluent.dll`

Load it when running PHP:

```sh
php -d extension=target/debug/libfluent.so example/hello-world.php    # Linux
php -d extension=target/debug/libfluent.dylib example/hello-world.php # macOS
php -d extension=target\debug\fluent.dll example\hello-world.php      # Windows
```

For a permanent installation, copy the built library into PHP's extension
directory and add an `extension=...` line to your `php.ini`.

## License

GPL-3.0. See [LICENSE](https://github.com/Ennexa/fluent-php/blob/master/LICENSE).
