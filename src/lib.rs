#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::boxed::ZBox;
use ext_php_rs::convert::{FromZval, IntoZval, IntoZvalDyn};
use ext_php_rs::flags::{DataType, IniEntryPermission};
use ext_php_rs::types::{ZendHashTable, Zval};
use ext_php_rs::{
    info_table_end, info_table_row, info_table_start,
    prelude::*,
    zend::{ce, ExecutorGlobals, IniEntryDef, ModuleEntry},
};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, Range};
use std::sync::Arc;

use fluent::types::FluentType;
use fluent::{FluentArgs, FluentBundle, FluentError, FluentResource, FluentValue};
use fluent_syntax::parser::ParserError;
use std::sync::{Mutex, MutexGuard};
use unic_langid::LanguageIdentifier;

mod cache;

// -- Exception classes --

#[php_class]
#[php(name = "FluentPHP\\Exception")]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
struct Exception;

#[php_class]
#[php(name = "FluentPHP\\ParserException")]
#[php(extends(Exception))]
#[derive(Default)]
struct ParserException {
    #[php(prop)]
    message: String,
    errors: Vec<(i64, i64, String)>,
}

#[php_impl]
impl ParserException {
    pub fn get_errors(&self) -> Vec<ZBox<ZendHashTable>> {
        self.errors
            .iter()
            .map(|(line, col, source)| {
                let mut ht = ZendHashTable::new();
                ht.insert("line", *line).unwrap();
                ht.insert("col", *col).unwrap();
                ht.insert("source", source.clone()).unwrap();
                ht
            })
            .collect()
    }
}

#[php_class]
#[php(name = "FluentPHP\\ResolverException")]
#[php(extends(Exception))]
#[derive(Default)]
struct ResolverException {
    #[php(prop)]
    message: String,
    errors: Vec<String>,
}

#[php_impl]
impl ResolverException {
    pub fn get_errors(&self) -> Vec<String> {
        self.errors.clone()
    }
}

#[php_class]
#[php(name = "FluentPHP\\CacheException")]
#[php(extends(Exception))]
#[derive(Default)]
struct CacheException;

// -- Internal error types --

#[derive(Debug)]
enum FluentPhpError {
    ParseError(Vec<FluentPhpParseError>),
    ResolverError(Vec<FluentError>),
    Message(String),
}

impl FluentPhpError {
    fn from_parse_error(resource: &FluentResource, errors: Vec<ParserError>) -> Self {
        let errors = errors
            .into_iter()
            .map(|err| FluentPhpParseError::new(resource, err))
            .collect();

        Self::ParseError(errors)
    }

    fn from_error(errors: Vec<FluentError>) -> Self {
        if errors.len() == 1 {
            return Self::Message(format!("{}", &errors[0]));
        }

        let ids = errors
            .iter()
            .map(|e| match e {
                FluentError::Overriding { id, .. } => format!("\"{}\"", id),
                _ => format!("{}", e),
            })
            .collect::<Vec<_>>();

        Self::Message(format!(
            "Attempt to override existing entries: {}.",
            ids.join(", ")
        ))
    }
}

fn resolver_inner(e: &FluentError) -> String {
    match e {
        FluentError::ResolverError(inner) => format!("{}", inner),
        _ => format!("{}", e),
    }
}

impl Display for FluentPhpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FluentPhpError::ParseError(errs) => {
                if errs.len() == 1 {
                    write!(f, "Parse error: {}", errs[0])
                } else {
                    write!(f, "Parse errors:")?;
                    for err in errs {
                        write!(f, "\n  - {}", err)?;
                    }
                    Ok(())
                }
            }
            FluentPhpError::ResolverError(errs) => {
                let count = errs.len();
                let label = if count == 1 {
                    "Resolution failed with error: ".to_string()
                } else {
                    format!("Resolution failed with {} errors: ", count)
                };
                let mut parts: Vec<String> = errs.iter().take(3).map(resolver_inner).collect();
                if count > 3 {
                    parts.push(format!("and {} more", count - 3));
                }
                write!(f, "{}{}", label, parts.join("; "))
            }
            FluentPhpError::Message(err) => write!(f, "{}", &err),
        }
    }
}

impl From<FluentPhpError> for PhpException {
    fn from(exception: FluentPhpError) -> Self {
        let message = format!("{}", exception);
        match exception {
            FluentPhpError::ParseError(parse_errors) => {
                let errors = parse_errors
                    .iter()
                    .map(|e| (e.line as i64, e.col as i64, e.source.clone()))
                    .collect();
                let obj = ParserException {
                    message: message.clone(),
                    errors,
                };
                PhpException::default(message).with_object(obj.into_zval(true).unwrap())
            }
            FluentPhpError::ResolverError(fluent_errors) => {
                let errors = fluent_errors.iter().map(resolver_inner).collect();
                let obj = ResolverException {
                    message: message.clone(),
                    errors,
                };
                PhpException::default(message).with_object(obj.into_zval(true).unwrap())
            }
            _ => PhpException::from_class::<Exception>(message),
        }
    }
}

fn cache_error_to_php(e: cache::CacheError) -> PhpException {
    match e {
        cache::CacheError::LockPoisoned => PhpException::from_class::<CacheException>(
            "The Fluent process cache is unavailable because its lock was poisoned.".to_string(),
        ),
        cache::CacheError::Io(io_err) => {
            PhpException::from_class::<Exception>(format!("I/O error: {}", io_err))
        }
        cache::CacheError::Parse { resource, errors } => {
            FluentPhpError::from_parse_error(&resource, errors).into()
        }
    }
}

fn cache_file_error_to_php(path: &str, e: cache::CacheError) -> PhpException {
    match e {
        cache::CacheError::Io(io_err) => PhpException::from_class::<Exception>(format!(
            "Failed to read file \"{}\": {}",
            path, io_err
        )),
        other => cache_error_to_php(other),
    }
}

// -- Parse error detail --

fn line_offset_from_range(str: &str, range: &Range<usize>) -> Option<(u32, usize)> {
    let mut bytes: usize = 0;

    for (line_no, line) in str.split('\n').enumerate() {
        let line_bytes = line.len() + 1;
        bytes += line_bytes;
        if bytes > range.start {
            return Some((line_no as u32 + 1, range.start + line_bytes - bytes));
        }
    }

    None
}

#[derive(Debug)]
struct FluentPhpParseError {
    line: u32,
    col: usize,
    source: String,
    error: ParserError,
}

impl Display for FluentPhpParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {}, col {}: {}", self.line, self.col, self.error)?;
        if !self.source.is_empty() {
            write!(f, " - \"{}\"", self.source.trim())?;
        }
        Ok(())
    }
}

impl FluentPhpParseError {
    fn new(resource: &FluentResource, error: ParserError) -> Self {
        let source = resource.source();
        let (line, col) = line_offset_from_range(source, &error.pos).unwrap_or_default();

        let slice = &error.slice;
        let source = slice.as_ref().map_or(String::new(), |range| {
            source[range.start..range.end].to_owned()
        });
        Self {
            line,
            col,
            source,
            error,
        }
    }
}

// -- FluentResource PHP class --

#[php_class]
#[php(name = "FluentPHP\\FluentResource")]
struct FluentPhpResource {
    inner: Arc<FluentResource>,
}

#[php_impl]
impl FluentPhpResource {
    pub fn from_string(source: String) -> PhpResult<Self> {
        let inner = cache::uncached_parse_string(source).map_err(cache_error_to_php)?;
        Ok(Self { inner })
    }

    pub fn from_file(path: String) -> PhpResult<Self> {
        let inner =
            cache::uncached_parse_file(&path).map_err(|e| cache_file_error_to_php(&path, e))?;
        Ok(Self { inner })
    }
}

// -- ResourceCache PHP class --

#[php_class]
#[php(name = "FluentPHP\\ResourceCache")]
#[derive(Default)]
struct ResourceCache;

#[php_impl]
impl ResourceCache {
    pub fn from_string(source: String) -> PhpResult<FluentPhpResource> {
        let inner = cache::get_or_parse_string(source).map_err(cache_error_to_php)?;
        Ok(FluentPhpResource { inner })
    }

    pub fn from_file(path: String) -> PhpResult<FluentPhpResource> {
        let inner =
            cache::get_or_parse_file(&path).map_err(|e| cache_file_error_to_php(&path, e))?;
        Ok(FluentPhpResource { inner })
    }

    pub fn invalidate_file(path: String) -> PhpResult<bool> {
        cache::invalidate_file(&path).map_err(cache_error_to_php)
    }

    pub fn clear() -> PhpResult<()> {
        cache::clear().map_err(cache_error_to_php)
    }

    pub fn get_stats() -> PhpResult<ZBox<ZendHashTable>> {
        let s = cache::stats().map_err(cache_error_to_php)?;
        let mut ht = ZendHashTable::new();
        ht.insert("entries", (s.string_entries + s.file_entries) as i64)
            .unwrap();
        ht.insert("cache_weight", s.current_weight as i64).unwrap();
        ht.insert("hits", s.hits as i64).unwrap();
        ht.insert("metadata_hits", s.metadata_hits as i64).unwrap();
        ht.insert("content_hits", s.content_hits as i64).unwrap();
        ht.insert("misses", s.misses as i64).unwrap();
        ht.insert("loads", s.loads as i64).unwrap();
        ht.insert("errors", s.errors as i64).unwrap();
        ht.insert("evictions", s.evictions as i64).unwrap();
        ht.insert("skipped_oversize", s.skipped_oversize as i64)
            .unwrap();
        ht.insert("max_weight", s.max_weight as i64).unwrap();
        ht.insert("pid", std::process::id() as i64).unwrap();
        Ok(ht)
    }
}

// -- Zval / FluentValue conversion --

fn zval_to_fluent_value(zv: Zval) -> FluentValue<'static> {
    if zv.is_string() {
        FluentValue::String(zv.string().unwrap().into())
    } else if zv.is_long() || zv.is_double() {
        FluentValue::Number(zv.double().unwrap().into())
    } else if zv.is_null() {
        FluentValue::None
    } else if zv.is_bool() {
        FluentValue::Custom(Box::new(FluentPhpBoolValue(zv.bool().unwrap())))
    } else if zv.is_object() {
        FluentValue::Custom(Box::new(FluentPhpObjectValue::new(zv.shallow_clone())))
    } else {
        FluentValue::Error
    }
}

#[derive(Debug)]
struct FluentPhpArgs<'a>(FluentArgs<'a>);

impl<'a> Deref for FluentPhpArgs<'a> {
    type Target = FluentArgs<'a>;

    fn deref(&self) -> &FluentArgs<'a> {
        &self.0
    }
}

impl<'a> TryFrom<&ZendHashTable> for FluentPhpArgs<'a> {
    type Error = FluentPhpError;

    fn try_from(value: &ZendHashTable) -> Result<Self, Self::Error> {
        let mut args = FluentArgs::new();
        for (key, elem) in value.iter() {
            let value = match FluentPhpValue::from_zval(elem) {
                Some(v) => v,
                None => {
                    return Err(FluentPhpError::Message(format!(
                        "Unsupported type for argument \"{}\": {}.",
                        key,
                        elem.get_type()
                    )))
                }
            };
            args.set(key.to_string(), value);
        }

        Ok(FluentPhpArgs(args))
    }
}

#[derive(Debug)]
struct ThreadSafeWrapper<T> {
    inner: Mutex<T>,
}

impl<T> ThreadSafeWrapper<T> {
    pub fn new(inner: T) -> Self {
        ThreadSafeWrapper {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.inner.lock().unwrap()
    }
}

impl<T> Deref for ThreadSafeWrapper<T> {
    type Target = Mutex<T>;

    fn deref(&self) -> &Mutex<T> {
        &self.inner
    }
}

#[derive(Debug)]
struct ThreadSafeZendCallable(ThreadSafeWrapper<ZendCallable<'static>>);

impl ThreadSafeZendCallable {
    fn new(callable: ZendCallable<'static>) -> Self {
        Self(ThreadSafeWrapper::new(callable))
    }

    fn lock(&self) -> MutexGuard<'_, ZendCallable<'static>> {
        self.0.lock()
    }
}

// Fluent stores functions behind `Send + Sync` trait objects. PHP callables are
// only invoked synchronously by the PHP request thread in this extension; the
// mutex prevents re-entrant mutable access if Fluent calls the function more
// than once during formatting.
unsafe impl Send for ThreadSafeZendCallable {}
unsafe impl Sync for ThreadSafeZendCallable {}

#[derive(Debug)]
struct FluentPhpObjectValue(ThreadSafeWrapper<Zval>);

impl FluentPhpObjectValue {
    fn new(zv: Zval) -> Self {
        Self(ThreadSafeWrapper::new(zv))
    }

    fn lock(&self) -> MutexGuard<'_, Zval> {
        self.0.lock()
    }

    fn stringify(&self) -> std::borrow::Cow<'static, str> {
        let zval = self.lock();
        if let Some(object) = zval.object() {
            if object.instance_of(ce::stringable()) {
                let result = object.try_call_method("__toString", vec![]);
                if let Ok(result) = result {
                    return result.str().unwrap().to_string().into();
                }
            }

            return "[Object]".into();
        }

        "[Object]".into()
    }

    fn object_identity(&self) -> Option<(usize, u32)> {
        let zval = self.lock();
        zval.object()
            .map(|object| (object.ce as usize, object.handle))
    }
}

// Fluent custom values must be `Send`, but PHP objects are raw Zend VM handles.
// The extension formats messages synchronously inside a PHP request; objects are
// mutex-protected and never intentionally moved to a Rust worker thread.
unsafe impl Send for FluentPhpObjectValue {}
unsafe impl Sync for FluentPhpObjectValue {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FluentPhpBoolValue(bool);

impl FluentPhpBoolValue {
    fn stringify(&self) -> std::borrow::Cow<'static, str> {
        if self.0 { "true" } else { "false" }.into()
    }

    fn to_php_value(self) -> FluentPhpValue {
        FluentPhpValue::Bool(self.0)
    }
}

impl PartialEq for FluentPhpObjectValue {
    fn eq(&self, other: &Self) -> bool {
        // Required by fluent-bundle's `FluentValue::Custom` equality support.
        // Current selector resolution does not compare custom object values; use
        // object identity here to avoid invoking PHP/Zend comparison handlers
        // from Rust `PartialEq`.
        self.object_identity()
            .zip(other.object_identity())
            .is_some_and(|(left, right)| left == right)
    }
}

impl FluentType for FluentPhpBoolValue {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(*self)
    }

    fn as_string(
        &self,
        _intls: &intl_memoizer::IntlLangMemoizer,
    ) -> std::borrow::Cow<'static, str> {
        self.stringify()
    }

    fn as_string_threadsafe(
        &self,
        _intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> std::borrow::Cow<'static, str> {
        self.stringify()
    }
}

impl FluentType for FluentPhpObjectValue {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(FluentPhpObjectValue::new(self.lock().shallow_clone()))
    }

    fn as_string(
        &self,
        _intls: &intl_memoizer::IntlLangMemoizer,
    ) -> std::borrow::Cow<'static, str> {
        self.stringify()
    }

    fn as_string_threadsafe(
        &self,
        _intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> std::borrow::Cow<'static, str> {
        self.stringify()
    }
}

enum FluentPhpValue {
    Bool(bool),
    Double(f64),
    Long(i64),
    Str(String),
    Zval(Zval),
    None,
}

impl Display for FluentPhpValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(s) => write!(f, "{}", &s),
            Self::Bool(true) => write!(f, "true"),
            Self::Bool(false) => write!(f, "false"),
            Self::Long(n) => write!(f, "{}", n),
            Self::Double(fl) => write!(f, "{}", fl),
            Self::None => write!(f, ""),
            Self::Zval(zv) => match zv {
                val if val.is_long() => write!(f, "{}", val.long().unwrap()),
                val if val.is_double() => write!(f, "{}", val.double().unwrap()),
                val if val.is_string() => write!(f, "{}", val.str().unwrap()),
                _ => write!(f, "{}", zv.str().unwrap_or("")),
            },
        }
    }
}

impl Clone for FluentPhpValue {
    fn clone(&self) -> Self {
        match self {
            Self::Str(val) => Self::Str(val.clone()),
            Self::Bool(val) => Self::Bool(*val),
            Self::Long(val) => Self::Long(*val),
            Self::Double(val) => Self::Double(*val),
            Self::None => Self::None,
            Self::Zval(val) => Self::Zval(val.shallow_clone()),
        }
    }
}

impl TryFrom<&FluentValue<'_>> for FluentPhpValue {
    type Error = FluentPhpError;

    fn try_from(value: &FluentValue<'_>) -> Result<Self, FluentPhpError> {
        let value = match value {
            FluentValue::String(s) => Self::Str(s.clone().into()),
            FluentValue::Number(n) => Self::Double(n.value),
            FluentValue::None => Self::None,
            FluentValue::Custom(val) => {
                if let Some(val) = val.as_ref().as_any().downcast_ref::<FluentPhpObjectValue>() {
                    FluentPhpValue::Zval(val.lock().shallow_clone())
                } else if let Some(val) = val.as_ref().as_any().downcast_ref::<FluentPhpBoolValue>()
                {
                    val.to_php_value()
                } else {
                    FluentPhpValue::None
                }
            }
            FluentValue::Error => {
                return Err(FluentPhpError::Message(
                    "Unsupported variable type".to_string(),
                ))
            }
        };

        Ok(value)
    }
}

impl From<FluentPhpValue> for FluentValue<'static> {
    fn from(value: FluentPhpValue) -> Self {
        match value {
            FluentPhpValue::Str(val) => Self::String(val.into()),
            FluentPhpValue::Long(val) => Self::Number(val.into()),
            FluentPhpValue::Double(val) => Self::Number(val.into()),
            FluentPhpValue::Bool(val) => Self::Custom(Box::new(FluentPhpBoolValue(val))),
            FluentPhpValue::Zval(val) => {
                Self::Custom(Box::new(FluentPhpObjectValue::new(val.shallow_clone())))
            }
            FluentPhpValue::None => Self::None,
        }
    }
}

impl FromZval<'_> for FluentPhpValue {
    const TYPE: DataType = DataType::Mixed;
    fn from_zval(zv: &Zval) -> Option<Self> {
        let val = if zv.is_string() {
            FluentPhpValue::Str(zv.string().unwrap())
        } else if zv.is_long() {
            FluentPhpValue::Long(zv.long().unwrap())
        } else if zv.is_double() {
            FluentPhpValue::Double(zv.double().unwrap())
        } else if zv.is_null() {
            FluentPhpValue::None
        } else if zv.is_bool() {
            FluentPhpValue::Bool(zv.bool().unwrap())
        } else if zv.is_object() {
            FluentPhpValue::Zval(zv.shallow_clone())
        } else {
            return None;
        };

        Some(val)
    }
}

impl IntoZval for FluentPhpValue {
    const TYPE: DataType = DataType::Mixed;
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, persistent: bool) -> ext_php_rs::error::Result<()> {
        match self {
            Self::Str(val) => zv.set_string(&val, persistent)?,
            Self::Bool(val) => zv.set_bool(val),
            Self::Long(val) => zv.set_long(val),
            Self::Double(val) => zv.set_double(val),
            Self::Zval(val) => *zv = val,
            Self::None => zv.set_null(),
        };
        Ok(())
    }
}

// -- FluentBundle PHP class --

#[php_class]
#[php(name = "FluentPHP\\FluentBundle")]
struct FluentPhpBundle {
    bundle: FluentBundle<Arc<FluentResource>>,
}

#[php_impl]
impl FluentPhpBundle {
    fn __construct(lang: String) -> PhpResult<Self> {
        let lang_id = match lang.parse::<LanguageIdentifier>() {
            Ok(lang_id) => lang_id,
            Err(_e) => {
                return Err(PhpException::from_class::<Exception>(
                    "Invalid language identifier.".to_string(),
                ))
            }
        };

        let mut bundle = FluentBundle::new(vec![lang_id]);

        bundle.set_use_isolating(false);
        Ok(Self { bundle })
    }

    pub fn add_resource(&mut self, resource: &Zval) -> PhpResult<()> {
        let arc = if resource.is_string() {
            let source = resource.string().ok_or_else(|| {
                PhpException::from_class::<Exception>("Failed to read string argument.".to_string())
            })?;
            cache::uncached_parse_string(source).map_err(cache_error_to_php)?
        } else if resource.is_object() {
            let obj = resource.object().ok_or_else(|| {
                PhpException::from_class::<Exception>("Failed to read object argument.".to_string())
            })?;
            let res: &FluentPhpResource = obj.extract().map_err(|_| {
                PhpException::from_class::<Exception>(
                    "addResource() expects a string or FluentResource instance.".to_string(),
                )
            })?;
            Arc::clone(&res.inner)
        } else {
            return Err(PhpException::from_class::<Exception>(
                "addResource() expects a string or FluentResource instance.".to_string(),
            ));
        };

        match self.bundle.add_resource(arc) {
            Ok(_) => Ok(()),
            Err(errors) => Err(FluentPhpError::from_error(errors).into()),
        }
    }

    pub fn add_function(&mut self, fn_name: String, callable: &Zval) -> PhpResult<()> {
        let callable = ZendCallable::new_owned(callable.shallow_clone()).unwrap();
        let callable = ThreadSafeZendCallable::new(callable);

        let status = self
            .bundle
            .add_function(&fn_name, move |oargs, _named_args| {
                let args: Result<Vec<FluentPhpValue>, FluentPhpError> =
                    oargs.iter().map(|p| p.try_into()).collect();

                if let Ok(args) = args {
                    let args: Vec<&dyn IntoZvalDyn> =
                        args.iter().map(|p| p as &dyn IntoZvalDyn).collect();

                    return callable
                        .lock()
                        .try_call(args)
                        .map(|value| zval_to_fluent_value(value))
                        .unwrap_or(FluentValue::Error);
                };

                FluentValue::Error
            });

        match status {
            Ok(_) => Ok(()),
            Err(e) => Err(PhpException::from_class::<Exception>(e.to_string())),
        }
    }

    pub fn format_pattern(&mut self, msg_id: String, arg_ids: &ZendHashTable) -> PhpResult<String> {
        let args: FluentPhpArgs = arg_ids.try_into()?;

        // Getting errors
        let mut errors = vec![];

        // Getting message
        let msg = match self.bundle.get_message(&msg_id) {
            Some(msg) => msg,
            None => {
                return Err(PhpException::from_class::<Exception>(format!(
                    "Message \"{}\" not found.",
                    msg_id
                )))
            }
        };

        // Formatting pattern
        let pattern = match msg.value() {
            Some(value) => value,
            None => {
                return Err(PhpException::from_class::<Exception>(format!(
                    "Message \"{}\" has no value.",
                    msg_id
                )))
            }
        };

        let value = self
            .bundle
            .format_pattern(pattern, Some(&args), &mut errors);

        if !errors.is_empty() {
            return Err(FluentPhpError::ResolverError(errors).into());
        }

        Ok(value.into_owned())
    }

    fn has_message(&mut self, msg_id: String) -> PhpResult<bool> {
        Ok(self.bundle.has_message(&msg_id))
    }
}

// -- Module info and startup --

#[no_mangle]
pub extern "C" fn php_module_info(_module: *mut ModuleEntry) {
    info_table_start!();
    info_table_row!("Fluent", "enabled");
    info_table_row!("Version", env!("CARGO_PKG_VERSION"));
    info_table_end!();
}

fn parse_ini_bool(s: &str) -> bool {
    !matches!(
        s.trim(),
        "" | "0" | "off" | "Off" | "OFF" | "false" | "False" | "FALSE" | "no" | "No" | "NO"
    )
}

extern "C" fn module_startup(_type: i32, module_number: i32) -> i32 {
    IniEntryDef::register(
        vec![
            IniEntryDef::new(
                "fluent.cache_enabled".to_string(),
                "1".to_string(),
                &IniEntryPermission::System,
            ),
            IniEntryDef::new(
                "fluent.cache_max_weight".to_string(),
                "16M".to_string(),
                &IniEntryPermission::System,
            ),
            IniEntryDef::new(
                "fluent.cache_max_entry_size".to_string(),
                "2M".to_string(),
                &IniEntryPermission::System,
            ),
            IniEntryDef::new(
                "fluent.cache_file_validation".to_string(),
                "metadata".to_string(),
                &IniEntryPermission::System,
            ),
        ],
        module_number,
    );

    let ini = ExecutorGlobals::get().ini_values();

    let enabled = match ini.get("fluent.cache_enabled") {
        Some(Some(v)) => parse_ini_bool(v),
        _ => true,
    };
    let max_weight = ini
        .get("fluent.cache_max_weight")
        .and_then(|o| o.as_deref())
        .and_then(cache::parse_memory_string);
    let max_entry_size = ini
        .get("fluent.cache_max_entry_size")
        .and_then(|o| o.as_deref())
        .and_then(cache::parse_memory_string);
    let file_validation = ini
        .get("fluent.cache_file_validation")
        .and_then(|o| o.as_deref())
        .and_then(|s| {
            if s.eq_ignore_ascii_case("checksum") {
                Some(cache::FileValidation::Checksum)
            } else if s.eq_ignore_ascii_case("metadata") {
                Some(cache::FileValidation::Metadata)
            } else {
                None
            }
        });

    if cache::configure_from_ini(enabled, max_weight, max_entry_size, file_validation).is_err() {
        return 1;
    }

    0
}

#[php_module]
#[php(startup = module_startup)]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<Exception>()
        .class::<ParserException>()
        .class::<ResolverException>()
        .class::<CacheException>()
        .class::<FluentPhpBundle>()
        .class::<FluentPhpResource>()
        .class::<ResourceCache>()
        .info_function(php_module_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_offset_from_range_reports_one_based_lines_and_zero_based_columns() {
        let source = "a\nbc\ndef";

        assert_eq!(line_offset_from_range(source, &(0..1)), Some((1, 0)));
        assert_eq!(line_offset_from_range(source, &(1..2)), Some((1, 1)));
        assert_eq!(line_offset_from_range(source, &(2..3)), Some((2, 0)));
        assert_eq!(line_offset_from_range(source, &(4..5)), Some((2, 2)));
        assert_eq!(line_offset_from_range(source, &(5..6)), Some((3, 0)));
        assert_eq!(line_offset_from_range(source, &(7..8)), Some((3, 2)));
        assert_eq!(line_offset_from_range(source, &(99..100)), None);
    }

    #[test]
    fn parse_ini_bool_treats_only_false_spellings_as_false() {
        for value in [
            "", "0", "off", "Off", "OFF", "false", "False", "FALSE", "no", "No", "NO", " 0 ",
            " off ", " false ", " no ",
        ] {
            assert!(!parse_ini_bool(value), "{value:?} should parse as false");
        }

        for value in ["1", "on", "true", "yes", "anything else", " falsey "] {
            assert!(parse_ini_bool(value), "{value:?} should parse as true");
        }
    }
}
