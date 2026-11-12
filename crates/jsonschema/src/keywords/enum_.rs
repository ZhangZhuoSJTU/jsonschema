use crate::{
    compiler,
    error::ValidationError,
    ext::cmp,
    keywords::CompilationResult,
    paths::{LazyLocation, Location, RefTracker},
    types::{JsonType, JsonTypeSet},
    validator::{Validate, ValidationContext},
};
use serde_json::{Map, Value};

#[derive(Debug)]
pub(crate) struct EnumValidator {
    options: Value,
    // Types that occur in items
    types: JsonTypeSet,
    items: Vec<Value>,
    location: Location,
}

impl EnumValidator {
    #[inline]
    pub(crate) fn compile<'a>(
        schema: &'a Value,
        items: &'a [Value],
        location: Location,
    ) -> CompilationResult<'a> {
        let mut types = JsonTypeSet::empty();
        for item in items {
            types = types.insert(JsonType::from(item));
        }
        Ok(Box::new(EnumValidator {
            options: schema.clone(),
            items: items.to_vec(),
            types,
            location,
        }))
    }
}

#[derive(Debug)]
pub(crate) struct SingleValueEnumValidator {
    value: Value,
    options: Value,
    location: Location,
}

impl SingleValueEnumValidator {
    #[inline]
    pub(crate) fn compile<'a>(
        schema: &'a Value,
        value: &'a Value,
        location: Location,
    ) -> CompilationResult<'a> {
        let options = match value {
            Value::Number(number) if number.as_f64().is_none() => value.clone(),
            _ => schema.clone(),
        };
        Ok(Box::new(SingleValueEnumValidator {
            options,
            value: value.clone(),
            location,
        }))
    }
}

#[inline]
fn enumeration_error<'i>(
    keyword_location: &Location,
    options: &Value,
    instance: &'i Value,
    instance_location: &LazyLocation,
    tracker: Option<&RefTracker>,
) -> ValidationError<'i> {
    ValidationError::enumeration(
        keyword_location.clone(),
        crate::paths::capture_evaluation_path(tracker, keyword_location),
        instance_location.into(),
        instance,
        options,
    )
}

#[inline]
pub(crate) fn compile<'a>(
    ctx: &compiler::Context,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    let location = ctx.location().join("enum");
    match schema {
        Value::Array(items) if items.len() == 1 => Some(SingleValueEnumValidator::compile(
            schema, &items[0], location,
        )),
        Value::Array(items) => Some(EnumValidator::compile(schema, items, location)),
        _ => Some(Err(ValidationError::single_type_error(
            location.clone(),
            location,
            Location::new(),
            schema,
            JsonType::Array,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"enum": [1]}), &json!(2), "/enum")]
    #[test_case(&json!({"enum": [1, 3]}), &json!(2), "/enum")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }
}

mod validation;
