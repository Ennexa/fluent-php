use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::SystemTime;

use fluent::FluentResource;
use fluent_syntax::parser::ParserError;
use lru::LruCache;
use xxhash_rust::xxh3::xxh3_128;

const DEFAULT_MAX_WEIGHT: usize = 16 * 1024 * 1024; // 16 MiB
const DEFAULT_MAX_ENTRY_SIZE: usize = 2 * 1024 * 1024; // 2 MiB
const WEIGHT_MULTIPLIER: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileValidation {
    Metadata,
    Checksum,
}

static CACHE_ENABLED: AtomicBool = AtomicBool::new(true);

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::new()));

// -- Error types --

#[derive(Debug)]
pub enum CacheError {
    LockPoisoned,
    Io(io::Error),
    /// A parse failure. The (boxed) resource is retained because the PHP layer
    /// uses its source to compute line/column offsets and snippets for the
    /// diagnostics it raises. Boxed to keep `CacheError` (and every
    /// `Result<_, CacheError>` on the happy path) small.
    Parse {
        resource: Box<FluentResource>,
        errors: Vec<ParserError>,
    },
}

impl From<io::Error> for CacheError {
    fn from(e: io::Error) -> Self {
        CacheError::Io(e)
    }
}

// -- Content key --

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ContentKey {
    len: u32,
    hash: u128,
}

fn content_key(source: &[u8]) -> ContentKey {
    ContentKey {
        len: source.len().try_into().unwrap_or(u32::MAX),
        hash: xxh3_128(source),
    }
}

// -- Unified LRU key --

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum CacheKey {
    String(ContentKey),
    File(PathBuf),
}

// -- Entry types --

struct StringEntry {
    resource: Arc<FluentResource>,
    estimated_weight: usize,
}

struct FileEntry {
    resource: Arc<FluentResource>,
    estimated_weight: usize,
    mtime: Option<SystemTime>,
    size: u64,
    content_hash: u128,
}

enum Entry {
    String(StringEntry),
    File(FileEntry),
}

impl Entry {
    fn weight(&self) -> usize {
        match self {
            Entry::String(e) => e.estimated_weight,
            Entry::File(e) => e.estimated_weight,
        }
    }
}

// -- Stats --

#[derive(Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub metadata_hits: u64,
    pub content_hits: u64,
    pub misses: u64,
    pub loads: u64,
    pub errors: u64,
    pub evictions: u64,
    pub skipped_oversize: u64,
    pub string_entries: u64,
    pub file_entries: u64,
    pub current_weight: usize,
    pub max_weight: usize,
}

// -- Cache --
//
// LRU order and stored data live in a single `LruCache<CacheKey, Entry>`.
// `current_weight` is the one piece of derived state we maintain by hand, and
// every mutation goes through `insert_entry` / `remove_entry` / `evict_for`,
// which are the only places that adjust it.

struct Cache {
    entries: LruCache<CacheKey, Entry>,
    max_weight: usize,
    max_entry_size: usize,
    file_validation: FileValidation,
    current_weight: usize,
    string_entries: u64,
    file_entries: u64,
    stats: CacheStats,
}

impl Cache {
    fn new() -> Self {
        Self {
            entries: LruCache::unbounded(),
            max_weight: DEFAULT_MAX_WEIGHT,
            max_entry_size: DEFAULT_MAX_ENTRY_SIZE,
            file_validation: FileValidation::Metadata,
            current_weight: 0,
            string_entries: 0,
            file_entries: 0,
            stats: CacheStats::default(),
        }
    }

    /// Insert (or replace) an entry, evicting LRU entries first to stay under
    /// `max_weight`. Callers must ensure `entry.weight() <= max_weight` (and
    /// `<= max_entry_size`) before calling — the oversize guard handles that.
    ///
    /// Remove-first: any existing entry at `key` is popped (and its weight
    /// reclaimed) before eviction runs, so replacing an entry never evicts
    /// other entries to make room the replacement does not actually need.
    fn insert_string_entry(&mut self, key: ContentKey, entry: StringEntry) {
        self.insert_entry(CacheKey::String(key), Entry::String(entry));
    }

    fn insert_file_entry(&mut self, path: PathBuf, entry: FileEntry) {
        self.insert_entry(CacheKey::File(path), Entry::File(entry));
    }

    fn insert_entry(&mut self, key: CacheKey, entry: Entry) {
        let weight = entry.weight();

        if let Some(old) = self.entries.pop(&key) {
            self.current_weight = self.current_weight.saturating_sub(old.weight());
            self.decrement_entry_count(&old);
        }

        self.evict_for(weight);

        self.increment_entry_count(&entry);
        self.entries.put(key, entry);
        self.current_weight = self.current_weight.saturating_add(weight);
    }

    fn remove_entry(&mut self, key: &CacheKey) -> bool {
        if let Some(entry) = self.entries.pop(key) {
            self.current_weight = self.current_weight.saturating_sub(entry.weight());
            self.decrement_entry_count(&entry);
            true
        } else {
            false
        }
    }

    fn evict_for(&mut self, needed: usize) {
        while self.current_weight.saturating_add(needed) > self.max_weight {
            let Some((_key, entry)) = self.entries.pop_lru() else {
                break;
            };
            self.current_weight = self.current_weight.saturating_sub(entry.weight());
            self.decrement_entry_count(&entry);
            self.stats.evictions += 1;
        }
    }

    fn increment_entry_count(&mut self, entry: &Entry) {
        match entry {
            Entry::String(_) => self.string_entries = self.string_entries.saturating_add(1),
            Entry::File(_) => self.file_entries = self.file_entries.saturating_add(1),
        }
    }

    fn decrement_entry_count(&mut self, entry: &Entry) {
        match entry {
            Entry::String(_) => {
                debug_assert!(
                    self.string_entries > 0,
                    "string entry count underflow while removing cache entry"
                );
                if self.string_entries > 0 {
                    self.string_entries -= 1;
                }
            }
            Entry::File(_) => {
                debug_assert!(
                    self.file_entries > 0,
                    "file entry count underflow while removing cache entry"
                );
                if self.file_entries > 0 {
                    self.file_entries -= 1;
                }
            }
        }
    }

    fn lookup_string(&mut self, key: &ContentKey) -> Option<Arc<FluentResource>> {
        let resource = match self.entries.get(&CacheKey::String(*key))? {
            Entry::String(e) => Arc::clone(&e.resource),
            // CacheKey::String always pairs with Entry::String at insertion.
            other => {
                debug_assert!(
                    matches!(other, Entry::String(_)),
                    "cache key/entry variant mismatch"
                );
                return None;
            }
        };
        Some(resource)
    }

    // LruCache lookup requires an owned CacheKey; cloning the canonical PathBuf
    // is the small cost of keeping file and string entries in one authoritative
    // LRU.
    fn lookup_file_by_metadata(
        &mut self,
        path: &Path,
        mtime: Option<SystemTime>,
        size: u64,
    ) -> Option<Arc<FluentResource>> {
        let resource = match self.entries.get(&CacheKey::File(path.to_path_buf()))? {
            Entry::File(e) if e.mtime == mtime && e.size == size => Arc::clone(&e.resource),
            // Metadata mismatch (stale), or variant mismatch (internal bug).
            _ => return None,
        };
        Some(resource)
    }

    fn lookup_file_by_content(
        &mut self,
        path: &Path,
        mtime: Option<SystemTime>,
        size: u64,
        hash: u128,
    ) -> Option<Arc<FluentResource>> {
        let resource = match self.entries.get_mut(&CacheKey::File(path.to_path_buf()))? {
            Entry::File(e) if e.content_hash == hash => {
                e.mtime = mtime;
                e.size = size;
                Arc::clone(&e.resource)
            }
            // Content hash mismatch (file changed), or variant mismatch (internal bug).
            _ => return None,
        };
        Some(resource)
    }

    fn get_stats(&self) -> CacheStats {
        let mut s = self.stats.clone();
        s.string_entries = self.string_entries;
        s.file_entries = self.file_entries;
        s.current_weight = self.current_weight;
        s.max_weight = self.max_weight;
        s
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.current_weight = 0;
        self.string_entries = 0;
        self.file_entries = 0;
        self.stats = CacheStats::default();
    }
}

// -- Helpers --

fn estimate_weight(source_len: usize) -> usize {
    source_len.saturating_mul(WEIGHT_MULTIPLIER)
}

/// Lexical path normalization (no filesystem access, does not resolve
/// symlinks). Used as a fallback for `invalidate_file` when the file no
/// longer exists (so `fs::canonicalize` would fail) and by the uncached
/// `FluentResource::fromFile` path.
pub fn normalize_path(p: &Path) -> PathBuf {
    let p = if p.is_relative() {
        std::env::current_dir()
            .map(|cwd| cwd.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    } else {
        p.to_path_buf()
    };
    let mut out = PathBuf::new();
    for component in p.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn canonicalize_existing_or_parent(p: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(p) {
        return canonical;
    }

    let normalized = normalize_path(p);
    let mut current = normalized.as_path();
    let mut suffix = PathBuf::new();

    loop {
        if let Ok(canonical) = fs::canonicalize(current) {
            return canonical.join(suffix);
        }

        let Some(name) = current.file_name() else {
            break;
        };
        let mut next_suffix = PathBuf::from(name);
        next_suffix.push(suffix);
        suffix = next_suffix;

        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }

    normalized
}

fn lock_cache() -> Result<MutexGuard<'static, Cache>, CacheError> {
    CACHE.lock().map_err(|_| CacheError::LockPoisoned)
}

fn bump_error(e: CacheError) -> CacheError {
    if let Ok(mut c) = lock_cache() {
        c.stats.errors += 1;
    }
    e
}

// -- Public API --

pub fn get_or_parse_string(source: String) -> Result<Arc<FluentResource>, CacheError> {
    if !is_cache_enabled() {
        return uncached_parse_string(source);
    }

    let key = content_key(source.as_bytes());
    let source_len = source.len();

    {
        let mut c = lock_cache()?;
        if let Some(hit) = c.lookup_string(&key) {
            c.stats.hits += 1;
            return Ok(hit);
        }
        c.stats.misses += 1;
    }

    let resource = match FluentResource::try_new(source) {
        Ok(r) => r,
        Err((r, errors)) => {
            if let Ok(mut c) = lock_cache() {
                c.stats.errors += 1;
            }
            return Err(CacheError::Parse {
                resource: Box::new(r),
                errors,
            });
        }
    };

    let arc = Arc::new(resource);
    let weight = estimate_weight(source_len);

    {
        let mut c = lock_cache()?;

        if let Some(existing) = c.lookup_string(&key) {
            return Ok(existing);
        }

        c.stats.loads += 1;

        // Entries larger than the per-entry limit are returned but never
        // cached, so they are re-parsed on every call. Size max_entry_size
        // accordingly.
        if weight > c.max_entry_size || weight > c.max_weight {
            c.stats.skipped_oversize += 1;
            return Ok(arc);
        }

        c.insert_string_entry(
            key,
            StringEntry {
                resource: Arc::clone(&arc),
                estimated_weight: weight,
            },
        );
    }

    Ok(arc)
}

pub fn get_or_parse_file(path: &str) -> Result<Arc<FluentResource>, CacheError> {
    if !is_cache_enabled() {
        return uncached_parse_file(path);
    }

    // Resolve symlinks and `..` against the real filesystem so the cache key
    // is canonical (two paths to the same file share one entry).
    let canonical = fs::canonicalize(path).map_err(|e| bump_error(e.into()))?;

    let meta = fs::metadata(&canonical).map_err(|e| bump_error(e.into()))?;
    let mtime = meta.modified().ok();
    let size = meta.len();

    {
        let mut c = lock_cache()?;
        if c.file_validation == FileValidation::Metadata {
            if let Some(hit) = c.lookup_file_by_metadata(&canonical, mtime, size) {
                c.stats.hits += 1;
                c.stats.metadata_hits += 1;
                return Ok(hit);
            }
        }
    }

    let content = fs::read_to_string(&canonical).map_err(|e| bump_error(e.into()))?;
    let hash = xxh3_128(content.as_bytes());

    {
        let mut c = lock_cache()?;
        if let Some(hit) = c.lookup_file_by_content(&canonical, mtime, size, hash) {
            c.stats.hits += 1;
            c.stats.content_hits += 1;
            return Ok(hit);
        }
        c.stats.misses += 1;
    }

    let source_len = content.len();
    let resource = match FluentResource::try_new(content) {
        Ok(r) => r,
        Err((r, errors)) => {
            if let Ok(mut c) = lock_cache() {
                c.stats.errors += 1;
            }
            return Err(CacheError::Parse {
                resource: Box::new(r),
                errors,
            });
        }
    };

    let arc = Arc::new(resource);
    let weight = estimate_weight(source_len);

    {
        let mut c = lock_cache()?;

        if let Some(existing) = c.lookup_file_by_content(&canonical, mtime, size, hash) {
            return Ok(existing);
        }

        c.stats.loads += 1;

        // See note in get_or_parse_string: oversized files are returned but
        // never cached, and are re-read and re-parsed on every call.
        if weight > c.max_entry_size || weight > c.max_weight {
            c.stats.skipped_oversize += 1;
            return Ok(arc);
        }

        c.insert_file_entry(
            canonical,
            FileEntry {
                resource: Arc::clone(&arc),
                estimated_weight: weight,
                mtime,
                size,
                content_hash: hash,
            },
        );
    }

    Ok(arc)
}

pub fn clear() -> Result<(), CacheError> {
    let mut c = lock_cache()?;
    c.clear();
    Ok(())
}

pub fn invalidate_file(path: &str) -> Result<bool, CacheError> {
    // Match the key used by get_or_parse_file. If the file is already gone,
    // canonicalize the nearest existing parent so symlinked prefixes such as
    // /tmp still resolve to the same stored key.
    let key = canonicalize_existing_or_parent(Path::new(path));
    let mut c = lock_cache()?;
    Ok(c.remove_entry(&CacheKey::File(key)))
}

pub fn stats() -> Result<CacheStats, CacheError> {
    let c = lock_cache()?;
    Ok(c.get_stats())
}

pub fn uncached_parse_string(source: String) -> Result<Arc<FluentResource>, CacheError> {
    match FluentResource::try_new(source) {
        Ok(r) => Ok(Arc::new(r)),
        Err((r, errors)) => Err(CacheError::Parse {
            resource: Box::new(r),
            errors,
        }),
    }
}

pub fn uncached_parse_file(path: &str) -> Result<Arc<FluentResource>, CacheError> {
    let normalized = normalize_path(Path::new(path));
    let content = fs::read_to_string(&normalized)?;
    match FluentResource::try_new(content) {
        Ok(r) => Ok(Arc::new(r)),
        Err((r, errors)) => Err(CacheError::Parse {
            resource: Box::new(r),
            errors,
        }),
    }
}

pub fn is_cache_enabled() -> bool {
    CACHE_ENABLED.load(Ordering::Relaxed)
}

/// Apply configuration read from php.ini. Intended to be called once at module
/// startup, before the cache is used — this is not a runtime-resize API, so it
/// does not evict to enforce a lowered cap (none is needed on an empty cache).
/// `None` leaves the corresponding default in place.
pub fn configure_from_ini(
    enabled: bool,
    max_weight: Option<usize>,
    max_entry_size: Option<usize>,
    file_validation: Option<FileValidation>,
) -> Result<(), CacheError> {
    CACHE_ENABLED.store(enabled, Ordering::Relaxed);
    let mut c = lock_cache()?;
    if let Some(w) = max_weight {
        c.max_weight = w;
    }
    if let Some(s) = max_entry_size {
        c.max_entry_size = s;
    }
    if let Some(v) = file_validation {
        c.file_validation = v;
    }
    Ok(())
}

pub fn parse_memory_string(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num_str, multiplier) = match s.as_bytes().last() {
        Some(b'G' | b'g') => (&s[..s.len() - 1], 1024 * 1024 * 1024),
        Some(b'M' | b'm') => (&s[..s.len() - 1], 1024 * 1024),
        Some(b'K' | b'k') => (&s[..s.len() - 1], 1024),
        _ => (s, 1),
    };
    num_str
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .map(|n| n.saturating_mul(multiplier))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn resource(source: &str) -> Arc<FluentResource> {
        Arc::new(FluentResource::try_new(source.to_string()).unwrap())
    }

    fn string_entry(source: &str, weight: usize) -> StringEntry {
        StringEntry {
            resource: resource(source),
            estimated_weight: weight,
        }
    }

    fn file_entry(source: &str, weight: usize, size: u64, hash: u128) -> FileEntry {
        FileEntry {
            resource: resource(source),
            estimated_weight: weight,
            mtime: None,
            size,
            content_hash: hash,
        }
    }

    #[test]
    fn replacing_entry_updates_weight_without_double_counting() {
        let mut cache = Cache::new();
        cache.max_weight = 100;

        let key = content_key(b"message = First");
        cache.insert_string_entry(key, string_entry("message = First", 30));
        cache.insert_string_entry(key, string_entry("message = Replacement", 20));

        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.current_weight, 20);
        assert_eq!(cache.string_entries, 1);
        assert_eq!(cache.file_entries, 0);
        assert_eq!(cache.stats.evictions, 0);
    }

    #[test]
    fn evicting_lru_entry_updates_weight_counts_and_stats() {
        let mut cache = Cache::new();
        cache.max_weight = 8;

        let first_key = content_key(b"first = First");
        let second_key = content_key(b"second = Second");
        let file_path = PathBuf::from("messages.ftl");

        cache.insert_string_entry(first_key, string_entry("first = First", 4));
        cache.insert_file_entry(file_path.clone(), file_entry("file = File", 4, 11, 0xfeed));
        cache.insert_string_entry(second_key, string_entry("second = Second", 4));

        assert!(cache.lookup_string(&first_key).is_none());
        assert!(cache.lookup_string(&second_key).is_some());
        assert!(cache
            .lookup_file_by_content(&file_path, None, 11, 0xfeed)
            .is_some());
        assert_eq!(cache.entries.len(), 2);
        assert_eq!(cache.current_weight, 8);
        assert_eq!(cache.string_entries, 1);
        assert_eq!(cache.file_entries, 1);
        assert_eq!(cache.stats.evictions, 1);
    }

    #[test]
    fn normalize_path_removes_current_and_parent_components() {
        let path = normalize_path(Path::new("alpha/./beta/../gamma"));

        assert!(path.ends_with(Path::new("alpha").join("gamma")));
        assert!(!path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir)));
    }

    #[test]
    fn canonicalize_existing_or_parent_preserves_missing_suffix() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "php-fluent-cache-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&base).unwrap();

        let missing_path = base.join("missing").join("messages.ftl");
        let expected = fs::canonicalize(&base)
            .unwrap()
            .join("missing")
            .join("messages.ftl");

        assert_eq!(canonicalize_existing_or_parent(&missing_path), expected);

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn parse_memory_string_accepts_units_and_rejects_invalid_values() {
        assert_eq!(parse_memory_string("42"), Some(42));
        assert_eq!(parse_memory_string(" 2 k "), Some(2 * 1024));
        assert_eq!(parse_memory_string("16M"), Some(16 * 1024 * 1024));
        assert_eq!(parse_memory_string("3g"), Some(3 * 1024 * 1024 * 1024));
        assert_eq!(parse_memory_string(""), None);
        assert_eq!(parse_memory_string("0"), None);
        assert_eq!(parse_memory_string("-1"), None);
        assert_eq!(parse_memory_string("1T"), None);
        assert_eq!(parse_memory_string("abc"), None);
    }

    #[test]
    fn parse_memory_string_saturates_on_overflow() {
        let huge_gigabytes = format!("{}G", usize::MAX);

        assert_eq!(parse_memory_string(&huge_gigabytes), Some(usize::MAX));
    }
}
