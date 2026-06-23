---
title: Cache
nav_order: 4
permalink: /cache/
---

# Resource Cache
{: .no_toc }

`ResourceCache` reuses parsed FTL across requests, so repeated requests skip
re-parsing. Cached files are reloaded automatically when the source changes.
{: .fs-6 .fw-300 }

<details open markdown="block">
  <summary>Table of contents</summary>
  {: .text-delta }
- TOC
{:toc}
</details>

---

## How it works

`FluentPhp\ResourceCache` keeps parsed `FluentResource` objects in memory,
within the current PHP process, looked up by source content (for strings) or
canonical file path (for files). It is designed for long-running PHP runtimes —
PHP-FPM, Swoole, RoadRunner, FrankenPHP — that serve many requests from the same
process.

- **Per-process.** Each worker process owns its own independent LRU cache.
  Entries are **not** shared between workers, and the cache does not survive a
  worker restart.
- **Automatic reload.** A cached file parse is reused until the file changes —
  detected by its modification time and size (`metadata` mode) or its content
  hash (`checksum` mode) — at which point the next `fromFile()` reparses and
  replaces the entry. See [validation modes](#file-validation-modes) for details.
- **LRU eviction.** When the cache exceeds `fluent.cache_max_weight`, the least
  recently used entries are evicted.
- **Weight-based, not count-based.** Each entry has an approximate weight derived
  from its source size; the cache bounds total weight rather than entry count.
- **String sources — identity by content hash.** A string source is identified by
  a 128-bit [xxh3](https://xxhash.com/) hash of its text. xxh3 is fast and
  non-cryptographic, so a collision — two different sources hashing alike and the
  wrong cached parse being returned — is possible in theory. In practice it is
  negligible: for 1,000 distinct sources the probability of *any* collision is
  about **1.5 × 10⁻³³**.
- **File sources — identity by path.** Files are looked up by canonical file path,
  not by a content hash, so the collision above never affects them — a file's hash is
  only used to detect whether it has changed (see
  [validation modes](#file-validation-modes)).

Typical flow:

**FTL file → ResourceCache → FluentResource → FluentBundle → formatted message**

```php
$resource = FluentPhp\ResourceCache::fromFile(__DIR__ . '/messages.ftl');

$bundle = new FluentPhp\FluentBundle('en');
$bundle->addResource($resource);

echo $bundle->formatPattern('checkout-title', []);
```

The file is parsed once and reused on later requests. In `metadata` and
`checksum` validation modes, you do not need to call `invalidateFile()` for
normal edits — changed files are reloaded automatically.

## INI settings

Configure the cache in `php.ini`. These are read once at module startup.

```ini
fluent.cache_enabled=1
fluent.cache_max_weight=16M
fluent.cache_max_entry_size=2M
fluent.cache_file_validation=metadata
```

| Setting | Default | Purpose |
|:--------|:--------|:--------|
| `fluent.cache_enabled` | `1` | Enable or disable `ResourceCache`. When disabled, lookups parse every time. |
| `fluent.cache_max_weight` | `16M` | Approximate total cache size before LRU eviction kicks in. |
| `fluent.cache_max_entry_size` | `2M` | Maximum approximate weight for a single entry. Larger resources are returned but never cached. |
| `fluent.cache_file_validation` | `metadata` | How file freshness is checked: `metadata` or `checksum`. |

Size values accept `K`, `M`, and `G` suffixes (e.g. `16M`, `1G`). A value of `0`
is treated as invalid and falls back to the default — disable caching with
`fluent.cache_enabled=0` instead.

### How the size is estimated

`max_weight` and `max_entry_size` bound *weight*, an approximation of in-memory
size rather than an exact byte count. Each entry's weight is estimated as roughly
**three times the source length**, since the parsed representation is larger than
the raw FTL. It is a deliberately rough heuristic, not a measurement of actual
process memory.

This matters when sizing the limits. For example, a ~700 KB FTL file has an
estimated weight of ~2.1 MB, which exceeds the default `max_entry_size` of `2M` —
so it would be parsed and returned but never cached (and counted under
`skipped_oversize` in [`getStats()`]({{ '/api-reference/#getstats' | relative_url }})).
Raise `max_entry_size` to cover your largest files, and `max_weight` to hold the
working set you expect each process to reuse.

## File validation modes

`fromFile()` must decide whether a cached parse is still fresh. Two modes trade
speed against change-detection strength:

`metadata` (default)
: Reuse a cached file resource when the canonical path, modification time, and
  size all match. The faster mode — no file read on a hit.

{: .warning }
> `metadata` can serve **stale content** if a file changes while preserving its
> modification time and size (for example, a same-size edit written within the
> filesystem's modification-time granularity). Use `checksum` when that risk
> matters.

`checksum`
: Read and hash the file on every `fromFile()` call before reusing a cached
  parse. Detects same-size content changes, at the cost of a full file read and
  hash on every call — including cache hits.

## Invalidation

`invalidateFile()` removes the cached entry for a path so the next `fromFile()`
reloads it:

```php
FluentPhp\ResourceCache::invalidateFile(__DIR__ . '/messages.ftl');
```

File changes are detected automatically, so explicit invalidation is rarely
needed — mainly to force an immediate reload, or to pick up a change that
`metadata` mode missed because the file kept the same size and modification time
(see [validation modes](#file-validation-modes)).

If the file has already been deleted, invalidation is **best-effort**: the
extension canonicalizes the nearest existing parent directory, which covers
common symlinked prefixes such as macOS `/tmp`.

`clear()` empties the entire cache for the current process. In both cases,
`FluentResource` objects and bundles you already hold remain valid.

### Scope: current process only

Like the cache itself, `invalidateFile()` and `clear()` affect **only the worker
process that runs them** — they do not reach other PHP-FPM, Swoole, or RoadRunner
workers. Calling either from a request refreshes just the one worker that
happened to handle it.

This rarely matters: every worker detects file changes on its own and reloads
independently, so a deploy is picked up across the whole fleet without any
invalidation call. `invalidateFile()` / `clear()` are best suited to
single-process contexts (CLI scripts, tests), to forcing an immediate reload in
the current worker, or to a runtime that can broadcast the call to all of its
workers. To refresh an entire pool at once, reload or recycle the workers on
deploy (a graceful PHP-FPM reload, `swoole reload`, `rr reset`, a container
redeploy, or `pm.max_requests`).

## Oversize resources

A resource whose estimated weight exceeds `fluent.cache_max_entry_size` (or the
total `max_weight`) is still parsed and returned, but it is **not cached** — it
will be re-parsed (and, for files, re-read) on every call, and counted under
`skipped_oversize` in [`getStats()`]({{ '/api-reference/#getstats' | relative_url }}).
Size `max_entry_size` to cover your largest translation files.

## Monitoring

`getStats()` returns per-process counters — hits, misses, evictions, current
weight, and more — useful for tuning cache size or confirming a worker is
actually reusing parses:

```php
$stats = FluentPhp\ResourceCache::getStats();

printf(
    "pid %d: %d entries, %d hits / %d misses, %d evictions\n",
    $stats['pid'],
    $stats['entries'],
    $stats['hits'],
    $stats['misses'],
    $stats['evictions'],
);
```

See the [API Reference]({{ '/api-reference/#getstats' | relative_url }}) for the
full list of keys.
