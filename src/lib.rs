#![cfg_attr(windows, feature(abi_vectorcall))]

use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, Range};
use ext_php_rs::types::{ZendHashTable};
use ext_php_rs::{
    info_table_end, info_table_row, info_table_start, prelude::*, zend::ModuleEntry,
};

use fluent::{FluentArgs, FluentBundle, FluentError, FluentResource, FluentValue};
use fluent_syntax::parser::ParserError;
use unic_langid::LanguageIdentifier;

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

    fn new(error: String) -> Self {
        Self::Message(error)
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
            if elem.is_string() {
                args.set(key, FluentValue::from(format!("{}", elem.str().unwrap())));
            } else if elem.is_long() || elem.is_double() {
                args.set(key, FluentValue::from(elem.double().unwrap()));
            } else {
                return Err(FluentPhpError::Message(format!("Invalid value for argument '{}'. Expected string or number.", key).into()));
            }
        }

        Ok(FluentPhpArgs(args))
    }
}#[php_impl(rename_methods = "camelCase")]
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
