#![cfg_attr(windows, feature(abi_vectorcall))]

use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, Range};
use ext_php_rs::convert::{FromZval, IntoZval, IntoZvalDyn};
use ext_php_rs::types::{ZendHashTable, Zval};
use ext_php_rs::{
    info_table_end, info_table_row, info_table_start, prelude::*, zend::ModuleEntry,
};
use ext_php_rs::flags::DataType;

use fluent::types::FluentType;
use fluent::{FluentArgs, FluentBundle, FluentError, FluentResource, FluentValue};
use fluent_syntax::parser::ParserError;
use unic_langid::LanguageIdentifier;
use std::sync::{Mutex, MutexGuard};


#[derive(Debug)]
enum FluentPhpError {
    ParseError(Vec<FluentPhpParseError>),
    Error(Vec<FluentError>),
    Message(String)
}

impl FluentPhpError {
    fn from_parse_error(resource: &FluentResource, errors: Vec<ParserError>) -> Self {
        let errors = errors.into_iter()
            .map(|err| FluentPhpParseError::new(resource, err))
            .collect();

        Self::ParseError(errors)
    }

    fn from_error(errors: Vec<FluentError>) -> Self {
        Self::Error(errors)
    }
}

impl Display for FluentPhpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
         match self {
            FluentPhpError::ParseError(err) => write!(f, "{}", &err[0]),
            FluentPhpError:: Error(err) => write!(f, "{}", &err[0]),
            FluentPhpError:: Message(err) => write!(f, "{}", &err),
        }
    }
}

impl From<FluentPhpError> for PhpException {
    fn from(exception: FluentPhpError) -> Self {
        PhpException::default(format!("{}", exception))
    }
}

fn line_offset_from_range(str: &str, range: &Range<usize>) -> Option<(u32, usize)> {
    let mut line_no:u32 = 1;
    let mut bytes: usize= 0;

    for line in str.lines() {
        let line_bytes = line.len() + 1;
        bytes += line_bytes;
        if bytes > range.start {
            return Some((line_no, range.start + line_bytes - bytes));
        }

        line_no += 1;
    }

    None
}

#[php_class(name = "FluentPHP\\FluentBundle")]
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
        write!(f, "{} on line {}, col {}\n\n{}\n\n", self.error, self.line, self.col, self.source)
    }
}

impl FluentPhpParseError {
    fn new(resource: &FluentResource, error: ParserError) -> Self {
        let source = resource.source();
        let (line, col) = match line_offset_from_range(source, &error.pos) {
            Some(val) => val,
            None => (0, 0)
        };

        let slice = &error.slice;
        let source = slice
            .as_ref()
            .map_or(String::new(), |range| source[range.start..range.end].to_owned());
        Self { line, col, source, error }
    }
}

fn zval_to_fluent_value(zv: Zval) -> FluentValue<'static>
{
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
        for (index, key, elem) in value.iter() {
            let key = match key {
                Some(key) => key,
                None => index.to_string(),
            };

            let key = format!("{}", key.as_str());
            let elem = FluentPhpValue::from_zval(elem);
            let value = match elem {
                Some(elem) => elem,
                None => return Err(FluentPhpError::Message(format!("Invalid value for argument '{}'. Expected string or number.", key).into()))
            };
            args.set(key, value);
        }

        Ok(FluentPhpArgs(args))
    }
}


#[derive(Debug)]
struct ThreadSafeWrapper<T>
{
    inner: Mutex<T>,
}

impl<T> ThreadSafeWrapper<T> {
    pub fn new(inner: T) -> Self {
        ThreadSafeWrapper {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.inner.lock().unwrap()
    }
}

impl<T> Deref for ThreadSafeWrapper<T>
{
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
            return format!("{}", zval.str().unwrap()).into();
        }
        if zval.is_double() || zval.is_long() {
            return format!("{}", zval.double().unwrap()).into();
        }

        if zval.is_bool() || zval.is_true() || zval.is_false() {
            return if zval.bool().unwrap() { "true" } else { "false" }.into();
        }

        if zval.is_object() {
            return "[Object]".into();
        }

        format!("Failed").into()
    }
}


impl Deref for FluentPhpZvalValue {
    type Target = ThreadSafeWrapper<Zval>;

    fn deref(&self) -> &ThreadSafeWrapper<Zval> {
        &self.0
    }
}

fn compare<T>(val1: Option<T>, val2: Option<T>) -> bool
    where T: PartialEq
{
    match (val1, val2) {
        (Some(val1), Some(val2)) => val1 == val2,
        _ => false
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
                    zo1.handlers.as_ref()
                        .and_then(|handlers| handlers.compare)
                        .map(|compare| (compare)(zv1.deref() as *const _ as *mut _, zv2.deref() as *const _ as *mut _))
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

    fn as_string(&self, _intls: &intl_memoizer::IntlLangMemoizer) -> std::borrow::Cow<'static, str> {
        return self.stringify();
    }

    fn as_string_threadsafe(
        &self,
        _intls: &intl_memoizer::concurrent::IntlLangMemoizer,
    ) -> std::borrow::Cow<'static, str> {
        return self.stringify();
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
            },
            FluentValue::Error => return Err(FluentPhpError::Message(format!("Unsupported value type for named parameter").into())),
        };

        Ok(value)
    }
}

impl Into<FluentValue<'_>> for FluentPhpValue
{
    fn into(self) -> FluentValue<'static> {
        match self {
            Self::Str(val) => FluentValue::String(val.into()),
            Self::Long(val) => FluentValue::Number(val.into()),
            Self::Double(val) => FluentValue::Number(val.into()),
            // Self::Bool(val) => FluentValue::Number(if val { 1 } else { 0 }.into()),
            Self::Zval(val) => FluentValue::Custom(Box::new(FluentPhpZvalValue::new(val.shallow_clone())))
,
            Self::None => FluentValue::None,
        }
    }
}

impl FromZval<'_> for FluentPhpValue {
    const TYPE: DataType = DataType::Mixed;
    fn from_zval(zv: &Zval) -> Option<Self> {
        let val = if zv.is_string() {
            FluentPhpValue::Str(zv.string().unwrap().into())
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

    fn set_zval(self, zv: &mut Zval, persistent: bool) -> ext_php_rs::error::Result<()> {
        match self.into() {
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

#[php_impl(rename_methods = "camelCase")]
impl FluentPhpBundle {
    #[constructor]
    fn __construct(lang: String) -> PhpResult<Self> {
        let lang_id = match lang.parse::<LanguageIdentifier>() {
            Ok(lang_id) => lang_id,
            Err(_e) => return Err("Invalid language identifier.".into()),
        };

        let mut bundle = FluentBundle::new(vec![lang_id]);

        bundle.set_use_isolating(false);
        Ok(Self { bundle })
    }

    #[php_method]
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
            Err(_error) => return Err(FluentPhpError::from_error(_error).into()),
        }
    }

    #[php_method]
    pub fn add_function(&mut self, fn_name: String, callable: &Zval) -> PhpResult<()> {
        let callable = ZendCallable::new_owned(callable.shallow_clone()).unwrap();
        let callable = ThreadSafeWrapper::new(callable);

        let status = self.bundle.add_function(&fn_name, move |oargs, _named_args| {

            // let val: dyn IntoZvalDyn = FluentPhpValue(args.get(0).unwrap());

            let args: Vec<FluentPhpValue> = oargs
                .iter()
                .map(|p| p.try_into().unwrap())
                .collect();
            let args: Vec<&dyn IntoZvalDyn> = args
                .iter()
                .map(|p| p as &dyn IntoZvalDyn)
                .collect();

            let value = callable.lock().try_call(args.into()).unwrap();

            return zval_to_fluent_value(value);
        });

        match status {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to add function".into()),
        }
    }

    #[php_method]
    fn format_pattern(&mut self, msg_id: String, arg_ids: &ZendHashTable) -> PhpResult<String> {
        let args:FluentPhpArgs = match arg_ids.try_into() {
            Ok(args) => args,
            Err(err) => return Err(err.into()),
        };

        // Getting errors
        let mut errors = vec![];

        // Getting message
        let msg = match self.bundle.get_message(&msg_id) {
            Some(msg) => msg,
            None => return Err("Message not found".into()),
        };

        // Formatting pattern
        let pattern = match msg.value() {
            Some(value) => value,
            None => return Err("Failed to load message AST.".into()),
        };

        let value = self
            .bundle
            .format_pattern(&pattern, Some(&args), &mut errors);

        Ok(value.into_owned())
    }
}

#[no_mangle]
pub extern "C" fn php_module_info(_module: *mut ModuleEntry) {
    info_table_start!();
    info_table_row!("Fluent", "enabled");
    info_table_end!();
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.info_function(php_module_info)
}
