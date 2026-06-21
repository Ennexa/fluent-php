#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::boxed::ZBox;
use ext_php_rs::convert::{FromZval, IntoZval, IntoZvalDyn};
use ext_php_rs::flags::DataType;
use ext_php_rs::types::{ZendHashTable, Zval};
use ext_php_rs::{
    info_table_end, info_table_row, info_table_start,
    prelude::*,
    zend::{ce, ModuleEntry},
};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, Range};

use fluent::types::FluentType;
use fluent::{FluentArgs, FluentBundle, FluentError, FluentResource, FluentValue};
use fluent_syntax::parser::ParserError;
use std::sync::{Mutex, MutexGuard};
use unic_langid::LanguageIdentifier;

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
                let mut parts: Vec<String> =
                    errs.iter().take(3).map(resolver_inner).collect();
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

#[php_class]
#[php(name = "FluentPHP\\FluentBundle")]
struct FluentPhpBundle {
    bundle: FluentBundle<FluentResource>,
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

fn zval_to_fluent_value(zv: Zval) -> FluentValue<'static> {
    if zv.is_string() {
        FluentValue::String(zv.string().unwrap().into())
    } else if zv.is_long() || zv.is_double() {
        FluentValue::Number(zv.double().unwrap().into())
    // } else if zv.is_bool() {
    //     FluentValue::Number(if zv.is_true() { 1 } else { 0 }.into())
    } else if zv.is_null() {
        FluentValue::None
    } else if zv.is_object() || zv.is_bool() {
        FluentValue::Custom(Box::new(FluentPhpZvalValue::new(zv.shallow_clone())))
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

unsafe impl<T> Send for ThreadSafeWrapper<T> {}

unsafe impl<T> Sync for ThreadSafeWrapper<T> {}

#[derive(Debug)]
struct FluentPhpZvalValue(ThreadSafeWrapper<Zval>);

impl FluentPhpZvalValue {
    pub fn new(zv: Zval) -> Self {
        Self(ThreadSafeWrapper::new(zv))
    }

    fn stringify(&self) -> std::borrow::Cow<'static, str> {
        let zval = self.0.lock();

        if zval.is_string() {
            return zval.str().unwrap().to_string().into();
        }

        if zval.is_double() || zval.is_long() {
            return format!("{}", zval.double().unwrap()).into();
        }

        if zval.is_bool() || zval.is_true() || zval.is_false() {
            return if zval.bool().unwrap() {
                "true"
            } else {
                "false"
            }
            .into();
        }

        if let Some(object) = zval.object() {
            if object.instance_of(ce::stringable()) {
                let result = object.try_call_method("__toString", vec![]);
                if let Ok(result) = result {
                    return result.str().unwrap().to_string().into();
                }
            }

            return "[Object]".into();
        }

        "Failed".to_string().into()
    }
}

impl Deref for FluentPhpZvalValue {
    type Target = ThreadSafeWrapper<Zval>;

    fn deref(&self) -> &ThreadSafeWrapper<Zval> {
        &self.0
    }
}

fn compare<T>(val1: Option<T>, val2: Option<T>) -> bool
where
    T: PartialEq,
{
    match (val1, val2) {
        (Some(val1), Some(val2)) => val1 == val2,
        _ => false,
    }
}

impl PartialEq for FluentPhpZvalValue {
    fn eq(&self, _other: &Self) -> bool {
        let zv1 = self.lock();
        let zv2 = _other.lock();

        if zv1.is_null() && zv2.is_null() {
            return true;
        }

        if zv1.is_bool() && zv2.is_bool() {
            return zv1.is_true() == zv2.is_true();
        }

        if zv1.is_long() && zv2.is_long() {
            return compare(zv1.long(), zv2.long());
        }

        if (zv1.is_long() || zv1.is_double()) && (zv2.is_long() || zv2.is_double()) {
            return compare(zv1.double(), zv2.double());
        }

        if zv1.is_string() && zv2.is_string() {
            return compare(zv1.str(), zv2.str());
        }

        if zv1.is_object() && zv2.is_object() {
            let zo1 = zv1.object();
            let zo2 = zv2.object();

            if let (Some(zo1), Some(zo2)) = (zo1, zo2) {
                if zo1.ce != zo2.ce {
                    return false;
                }

                let status = unsafe {
                    zo1.handlers
                        .as_ref()
                        .and_then(|handlers| handlers.compare)
                        .map(|compare| {
                            (compare)(
                                zv1.deref() as *const _ as *mut _,
                                zv2.deref() as *const _ as *mut _,
                            )
                        })
                };

                if let Some(status) = status {
                    return 0 == status;
                }
            }
        }

        false
    }
}

impl FluentType for FluentPhpZvalValue {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(FluentPhpZvalValue::new(self.0.lock().shallow_clone()))
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
    // Bool(bool),
    Double(f64),
    Long(i64),
    Str(String),
    Zval(Zval),
    None,
    // Error,
}

impl Display for FluentPhpValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(s) => write!(f, "{}", &s),
            Self::Long(n) => write!(f, "{}", n),
            Self::Double(fl) => write!(f, "{}", fl),
            // Self::Bool(true) => write!(f, "{}", "true"),
            // Self::Bool(false) => write!(f, "{}", "false"),
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
            Self::Long(val) => Self::Long(*val),
            Self::Double(val) => Self::Double(*val),
            // Self::Bool(val) => Self::Bool(*val),
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
                if let Some(val) = val.as_ref().as_any().downcast_ref::<FluentPhpZvalValue>() {
                    FluentPhpValue::Zval(val.lock().shallow_clone())
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
            // FluentPhpValue::Bool(val) => Self::Number(if val { 1 } else { 0 }.into()),
            FluentPhpValue::Zval(val) => {
                Self::Custom(Box::new(FluentPhpZvalValue::new(val.shallow_clone())))
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
        // } else if zv.is_bool() {
        //     FluentPhpValue::Bool(zv.bool().unwrap())
        } else if zv.is_null() {
            FluentPhpValue::None
        } else if zv.is_object() || zv.is_bool() {
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
            Self::Long(val) => zv.set_long(val),
            // Self::Bool(val) => zv.set_bool(val),
            Self::Double(val) => zv.set_double(val),
            Self::Zval(val) => *zv = val,
            Self::None => zv.set_null(),
        };
        Ok(())
    }
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

    pub fn add_resource(&mut self, source: String) -> PhpResult<()> {
        // Initializing resource
        let resource = match FluentResource::try_new(source) {
            Ok(resource) => resource,
            Err((_resource, _error)) => {
                return Err(FluentPhpError::from_parse_error(&_resource, _error).into())
            }
        };

        let bundle = &mut self.bundle;
        match bundle.add_resource(resource) {
            Ok(_value) => Ok(()),
            Err(_error) => Err(FluentPhpError::from_error(_error).into()),
        }
    }

    pub fn add_function(&mut self, fn_name: String, callable: &Zval) -> PhpResult<()> {
        let callable = ZendCallable::new_owned(callable.shallow_clone()).unwrap();
        let callable = ThreadSafeWrapper::new(callable);

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

#[no_mangle]
pub extern "C" fn php_module_info(_module: *mut ModuleEntry) {
    info_table_start!();
    info_table_row!("Fluent", "enabled");
    info_table_row!("Version", env!("CARGO_PKG_VERSION"));
    info_table_end!();
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<Exception>()
        .class::<ParserException>()
        .class::<ResolverException>()
        .class::<FluentPhpBundle>()
        .info_function(php_module_info)
}
