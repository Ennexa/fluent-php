<?php

namespace FluentPhp
{
    class Exception extends \Exception {}

    class ParserException extends Exception
    {
        /**
         * @return array<array{line: int, col: int, source: string}>
         */
        public function getErrors(): array {}
    }

    class ResolverException extends Exception
    {
        /**
         * @return array<string>
         */
        public function getErrors(): array {}
    }

    class CacheException extends Exception {}

    /**
     * A parsed FTL resource that can be added to one or more bundles.
     */
    final class FluentResource
    {
        /**
         * Parse an FTL source string without using the process cache.
         *
         * @throws ParserException if the FTL source contains syntax errors
         */
        public static function fromString(string $source): self {}

        /**
         * Read and parse an FTL file without using the process cache.
         *
         * @throws ParserException if the FTL file contains syntax errors
         * @throws Exception if the file cannot be read
         */
        public static function fromFile(string $path): self {}
    }

    /**
     * A cache of parsed FluentResource objects, kept in memory within the
     * current PHP process. Each worker process maintains its own independent
     * cache; entries are NOT shared between workers.
     *
     * clear() and invalidateFile() affect only the worker process handling the
     * call. They do NOT clear caches in other PHP-FPM, Swoole, RoadRunner, or
     * FrankenPHP workers.
     *
     * Useful for long-running PHP runtimes (PHP-FPM, Swoole, RoadRunner,
     * FrankenPHP) that serve many requests from the same process.
     *
     * INI settings:
     * - fluent.cache_enabled: bool, default 1
     * - fluent.cache_max_weight: memory size, default 16M
     * - fluent.cache_max_entry_size: memory size, default 2M
     * - fluent.cache_file_validation: metadata|checksum, default metadata.
     *   metadata trusts path, modification time, and size, and is the faster
     *   mode, but can serve stale content if a file changes without those values
     *   changing. checksum reads and hashes the file on every fromFile() call
     *   before reusing a cached parse.
     *
     * All methods are static; this class is not meant to be instantiated.
     */
    final class ResourceCache
    {
        /**
         * Return a parsed resource cached by source-content identity.
         *
         * Source identity is based on a 128-bit content hash.
         *
         * @throws ParserException if the FTL source contains syntax errors
         * @throws CacheException if the cache is unavailable
         */
        public static function fromString(string $source): FluentResource {}

        /**
         * Return a parsed resource cached by file path.
         *
         * By default, file changes are detected using path, size, and
         * modification time. Set fluent.cache_file_validation=checksum to read
         * and hash the file before reusing a cached parse.
         *
         * @throws ParserException if the FTL file contains syntax errors
         * @throws Exception if the file cannot be read
         * @throws CacheException if the cache is unavailable
         */
        public static function fromFile(string $path): FluentResource {}

        /**
         * Invalidate a cached file entry.
         *
         * Affects only the worker process handling this call. After
         * invalidation, the next fromFile() call in this worker reloads the file.
         * Existing FluentResource objects and bundles remain valid.
         */
        public static function invalidateFile(string $path): bool {}

        /**
         * Remove all entries from the cache.
         *
         * Affects only the worker process handling this call.
         *
         * Existing FluentResource objects and bundles remain valid.
         *
         * @throws CacheException if the cache is unavailable
         */
        public static function clear(): void {}

        /**
         * Return cache statistics for the current process.
         *
         * @return array{
         *     entries: int,
         *     cache_weight: int,
         *     hits: int,
         *     metadata_hits: int,
         *     content_hits: int,
         *     misses: int,
         *     loads: int,
         *     errors: int,
         *     evictions: int,
         *     skipped_oversize: int,
         *     max_weight: int,
         *     pid: int
         * }
         * @throws CacheException if the cache is unavailable
         */
        public static function getStats(): array {}

    }

    class FluentBundle
    {
        /**
         * @throws Exception if the language identifier is invalid
         */
        public function __construct(string $langCode) {}

        /**
         * Add a parsed resource to the bundle.
         *
         * Accepts either a FluentResource object or a raw FTL string
         * (for backward compatibility). Strings are parsed inline
         * without caching.
         *
         * @param string|FluentResource $resource
         * @throws ParserException if a string argument contains syntax errors
         * @throws Exception if any entry in the resource duplicates an existing one
         */
        public function addResource(string|FluentResource $resource): void {}

        /**
         * @param callable(): mixed $callable
         * @throws Exception if a function with that name is already registered
         */
        public function addFunction(string $name, callable $callable): void {}

        /**
         * @param array<string, mixed> $parameters
         * @throws Exception if the message is not found or has no value, or an argument type is unsupported
         * @throws ResolverException if the pattern references undefined variables or functions
         */
        public function formatPattern(string $messageId, array $parameters): string {}

        public function hasMessage(string $messageId): bool {}
    }
}
