use pyo3::{
    prelude::*,
    types::{PyDict, PySet},
};

use crate::{
    build_tools::{is_strict, SchemaDict},
    errors::{as_internal, context, err_val_error, ErrorKind},
    input::{GenericSequence, Input},
};

use super::{
    any::AnyValidator, build_validator, BuildContext, BuildValidator, CombinedValidator, Extra, ValResult, Validator,
};

#[derive(Debug, Clone)]
pub struct SetValidator {
    strict: bool,
    item_validator: Box<CombinedValidator>,
    min_items: Option<usize>,
    max_items: Option<usize>,
}

impl BuildValidator for SetValidator {
    const EXPECTED_TYPE: &'static str = "set";

    fn build(
        schema: &PyDict,
        config: Option<&PyDict>,
        build_context: &mut BuildContext,
    ) -> PyResult<CombinedValidator> {
        Ok(Self {
            strict: is_strict(schema, config)?,
            item_validator: match schema.get_item("items") {
                Some(d) => Box::new(build_validator(d, config, build_context)?.0),
                None => Box::new(AnyValidator::build(schema, config, build_context)?),
            },
            min_items: schema.get_as("min_items")?,
            max_items: schema.get_as("max_items")?,
        }
        .into())
    }
}

impl Validator for SetValidator {
    fn validate<'s, 'data, I: Input<'data>>(
        &'s self,
        py: Python<'data>,
        input: &'data I,
        extra: &Extra,
        slots: &'data [CombinedValidator],
    ) -> ValResult<'data, PyObject> {
        let set = match self.strict {
            true => input.strict_set()?,
            false => input.lax_set()?,
        };
        self._validation_logic(py, input, set, extra, slots)
    }

    fn validate_strict<'s, 'data, I: Input<'data>>(
        &'s self,
        py: Python<'data>,
        input: &'data I,
        extra: &Extra,
        slots: &'data [CombinedValidator],
    ) -> ValResult<'data, PyObject> {
        self._validation_logic(py, input, input.strict_set()?, extra, slots)
    }

    fn get_name(&self, py: Python) -> String {
        format!("{}-{}", Self::EXPECTED_TYPE, self.item_validator.get_name(py))
    }
}

impl SetValidator {
    fn _validation_logic<'s, 'data, I: Input<'data>>(
        &'s self,
        py: Python<'data>,
        input: &'data I,
        list: GenericSequence<'data>,
        extra: &Extra,
        slots: &'data [CombinedValidator],
    ) -> ValResult<'data, PyObject> {
        let length = list.generic_len();
        if let Some(min_length) = self.min_items {
            if length < min_length {
                return err_val_error!(
                    input_value = input.as_error_value(),
                    kind = ErrorKind::SetTooShort,
                    context = context!("min_length" => min_length)
                );
            }
        }
        if let Some(max_length) = self.max_items {
            if length > max_length {
                return err_val_error!(
                    input_value = input.as_error_value(),
                    kind = ErrorKind::SetTooLong,
                    context = context!("max_length" => max_length)
                );
            }
        }

        let output = list.validate_to_vec(py, length, &self.item_validator, extra, slots)?;
        Ok(PySet::new(py, &output).map_err(as_internal)?.into_py(py))
    }
}
