# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-23

### ⚠ Breaking changes

- Renamed the PHP namespace from `FluentPHP\` to `FluentPhp\`. Update all
  references — for example, `FluentPHP\FluentBundle` becomes
  `FluentPhp\FluentBundle`.

### Added

- `FluentPhp\ResourceCache` — a per-process, in-memory cache of parsed resources,
  with LRU eviction and configurable size limits. Methods: `fromString()`,
  `fromFile()`, `invalidateFile()`, `clear()`, and `getStats()`. Configurable via
  `php.ini`: `fluent.cache_enabled`, `fluent.cache_max_weight`,
  `fluent.cache_max_entry_size`, and `fluent.cache_file_validation`.
- `FluentPhp\FluentResource` — a reusable parsed resource (`fromString()`,
  `fromFile()`) that can be added to multiple bundles.
- Cached-file change detection via `metadata` (default) or `checksum` validation
  modes (`fluent.cache_file_validation`).
- Exception hierarchy: `FluentPhp\ParserException` and
  `FluentPhp\ResolverException` (both exposing `getErrors()`), and
  `FluentPhp\CacheException`, all extending `FluentPhp\Exception`.
- `FluentBundle::addResource()` now also accepts a `FluentResource` object.
  Raw FTL strings continue to work (parsed inline, without caching).
- Documentation site (GitHub Pages) under `docs/`.

### Changed

- Reduced `unsafe` code in the extension internals.
- CI: added dependency caching, linting, and testing; build matrix across
  PHP 8.2–8.5 (NTS and TS) and multiple targets.

### Internal

- Dependency updates and additional tests.

## [0.1.1] - 2026-03-06

### Added

- PHP 8.4 and 8.5 support.

### Removed

- PHP 8.1 support.

### Fixed

- Lifetime warning.

### Changed

- Dependency updates and CI cleanup.

## [0.1.0] - 2024-07-15

Initial release.

### Added

- `FluentBundle` for parsing Fluent (`.ftl`) resources and formatting messages:
  `addResource()`, `formatPattern()`, and `hasMessage()`.
- Custom Fluent functions via `addFunction()` (PHP callables).
- Message parameters as strings, numbers, booleans, `null`, and objects;
  `Stringable` objects are rendered through `__toString()`.
- Fluent selectors.
- Structured parse and resolution error reporting.
- PHP 8.2 and 8.3 support.

[0.2.0]: https://github.com/Ennexa/fluent-php/releases/tag/v0.2.0
[0.1.1]: https://github.com/Ennexa/fluent-php/releases/tag/v0.1.1
[0.1.0]: https://github.com/Ennexa/fluent-php/releases/tag/v0.1.0
