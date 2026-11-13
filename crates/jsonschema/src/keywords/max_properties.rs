#![allow(clippy::float_cmp, clippy::cast_sign_loss)]

use crate::{
    compiler,
    error::ValidationError,
    keywords::{helpers::fail_on_non_positive_integer, CompilationResult},
    paths::{LazyLocation, Location, RefTracker},
    validator::{Validate, ValidationContext},
};
use serde_json::{Map, Value};

pub(crate) struct MaxPropertiesValidator {
    limit: u64,
    location: Location,
}

impl MaxPropertiesValidator {
    #[inline]
    pub(crate) fn compile<'a>(
        ctx: &compiler::Context,
        schema: &'a Value,
        location: Location,
    ) -> CompilationResult<'a> {
        // Plain non-negative integers are taken as is; integer-valued floats
        // (e.g. `2.0`) are accepted only by drafts that allow them.
        let limit = match schema.as_u64() {
            Some(limit) => Some(limit),
            None if ctx.supports_integer_valued_numbers() => schema
                .as_f64()
                .filter(|value| value.trunc() == *value)
                // NOTE: Imprecise cast as big integers are not supported yet
                .map(|value| {
                    #[allow(clippy::cast_possible_truncation)]
                    let truncated = value as u64;
                    truncated
                }),
            None => None,
        };
        match limit {
            Some(limit) => Ok(Box::new(MaxPropertiesValidator { limit, location })),
            None => Err(fail_on_non_positive_integer(schema, location)),
        }
    }
}

impl Validate for MaxPropertiesValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        let size = match instance {
            Value::Object(item) => item.keys().fold(0u64, |seen, _| seen + 1),
            _ => return true,
        };
        !(size > self.limit)
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let Value::Object(item) = instance else {
            return Ok(());
        };
        if (item.len() as u64) <= self.limit {
            return Ok(());
        }
        Err(ValidationError::max_properties(
            self.location.clone(),
            crate::paths::capture_evaluation_path(tracker, &self.location),
            location.into(),
            instance,
            self.limit,
        ))
    }
}

#[inline]
pub(crate) fn compile<'a>(
    ctx: &compiler::Context,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    let location = ctx.location().join("maxProperties");
    Some(MaxPropertiesValidator::compile(ctx, schema, location))
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(
            &json!({"maxProperties": 1}),
            &json!({"a": 1, "b": 2}),
            "/maxProperties",
        );
    }
}
