#![cfg_attr(windows, feature(abi_vectorcall))]

use std::ops::Range;
use ext_php_rs::types::ZendHashTable;
use ext_php_rs::{
    info_table_end, info_table_row, info_table_start, prelude::*, zend::ModuleEntry,
};

use fluent::{FluentArgs, FluentBundle, FluentError, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

#[php_class(name = "FluentPHP\\Exception\\ParserException")]
#[extends(ext_php_rs::zend::ce::exception())]
#[derive(Default, Clone)]
pub struct FluentPhpException {
    errors: Vec<FluentError>
}
impl From<FluentPhpException> for PhpException {
    fn from(exception: FluentPhpException) -> Self {
        let msg = format!("{}", &exception.errors[0]);

        PhpException::default(msg.into())
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
        let source_copy = String::from(&source);
        let resource = match FluentResource::try_new(source) {
            Ok(resource) => resource,
            Err((_resource, _error)) => {
                let slice = _error[0].slice
                    .as_ref()
                    .map_or(String::new(), |range| {
                        let line = line_offset_from_range(&source_copy, range)
                            .map_or(String::new(), |(line, _)| format!(" on line {}", line));

                       format!("{}\n\n{}\n\n", line, source_copy[range.start..range.end].to_owned())
                    });

                return Err(format!("{}{}", _error[0], slice).into())
            }
        };

        let bundle = &mut self.bundle;
        match bundle.add_resource(resource) {
            Ok(_value) => Ok(()),
            Err(_error) => return Err(FluentPhpException{errors: _error}.into()),
        }
    }

    #[php_method]
    fn format_pattern(&mut self, msg_id: String, arg_ids: &ZendHashTable) -> PhpResult<String> {
        let mut args = FluentArgs::new();
        for (index, key, elem) in arg_ids.iter() {
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
                return Err(format!(
                    "Invalid value for argument '{}'. Expected string or number.",
                    key
                )
                .into());
            }
        }

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
