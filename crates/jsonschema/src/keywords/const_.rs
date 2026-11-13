use crate::{
    compiler,
    error::ValidationError,
    ext::cmp,
    keywords::CompilationResult,
    paths::Location,
    validator::{Validate, ValidationContext},
};
use serde_json::{Map, Number, Value};

use crate::paths::{LazyLocation, RefTracker};

struct ConstArrayValidator {
    value: Vec<Value>,
    location: Location,
}
impl ConstArrayValidator {
    #[inline]
    pub(crate) fn compile(value: &[Value], location: Location) -> CompilationResult<'_> {
        Ok(Box::new(ConstArrayValidator {
            value: value.to_vec(),
            location,
        }))
    }
}
impl Validate for ConstArrayValidator {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let mut outcome = Ok(());
        let mut pending = 1u8;
        while pending != 0 {
            pending -= 1;
            if self.is_valid(instance, ctx) {
                continue;
            }
            let here = &self.location;
            outcome = Err(ValidationError::constant_array(
                here.clone(),
                crate::paths::capture_evaluation_path(tracker, here),
                location.into(),
                instance,
                &self.value,
            ));
        }
        outcome
    }

    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        let candidate = match instance {
            Value::Array(instance_value) => Some(instance_value),
            _ => None,
        };
        let mut differs = true;
        if let Some(instance_value) = candidate {
            differs = !cmp::equal_arrays(&self.value, instance_value);
        }
        !differs
    }
}

struct ConstBooleanValidator {
    value: bool,
    location: Location,
}
impl ConstBooleanValidator {
    #[inline]
    pub(crate) fn compile<'a>(value: bool, location: Location) -> CompilationResult<'a> {
        Ok(Box::new(ConstBooleanValidator { value, location }))
    }
}
impl Validate for ConstBooleanValidator {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        match (self.is_valid(instance, ctx), &self.location) {
            (false, here) => Err(ValidationError::constant_boolean(
                Location::clone(here),
                crate::paths::capture_evaluation_path(tracker, here),
                location.into(),
                instance,
                self.value,
            )),
            (true, _) => Ok(()),
        }
    }

    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        let expected = u8::from(self.value);
        match instance {
            Value::Bool(instance_value) => (expected ^ u8::from(*instance_value)) == 0,
            _ => expected > 1,
        }
    }
}

struct ConstNullValidator {
    location: Location,
}
impl ConstNullValidator {
    #[inline]
    pub(crate) fn compile<'a>(location: Location) -> CompilationResult<'a> {
        Ok(Box::new(ConstNullValidator { location }))
    }
}
impl Validate for ConstNullValidator {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let rejected = !self.is_valid(instance, ctx);
        let failure = rejected.then(|| {
            ValidationError::constant_null(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
            )
        });
        failure.map_or(Ok(()), Err)
    }
    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        !matches!(
            instance,
            Value::Bool(_)
                | Value::Number(_)
                | Value::String(_)
                | Value::Array(_)
                | Value::Object(_)
        )
    }
}

struct ConstNumberValidator {
    // This is saved in order to ensure that the error message is not altered by precision loss
    original_value: Number,
    location: Location,
}

impl ConstNumberValidator {
    #[inline]
    pub(crate) fn compile(original_value: &Number, location: Location) -> CompilationResult<'_> {
        Ok(Box::new(ConstNumberValidator {
            original_value: original_value.clone(),
            location,
        }))
    }
}

impl Validate for ConstNumberValidator {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let mut outcome = Ok(());
        let mut pending = 1u8;
        while pending != 0 {
            pending -= 1;
            if self.is_valid(instance, ctx) {
                continue;
            }
            let here = &self.location;
            outcome = Err(ValidationError::constant_number(
                here.clone(),
                crate::paths::capture_evaluation_path(tracker, here),
                location.into(),
                instance,
                &self.original_value,
            ));
        }
        outcome
    }

    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        let mut answer = false;
        if let Value::Number(item) = instance {
            answer |= cmp::equal_numbers(item, &self.original_value);
        }
        answer
    }
}

pub(crate) struct ConstObjectValidator {
    value: Map<String, Value>,
    location: Location,
}

impl ConstObjectValidator {
    #[inline]
    pub(crate) fn compile(value: &Map<String, Value>, location: Location) -> CompilationResult<'_> {
        Ok(Box::new(ConstObjectValidator {
            value: value.clone(),
            location,
        }))
    }
}

impl Validate for ConstObjectValidator {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        match (self.is_valid(instance, ctx), &self.location) {
            (false, here) => Err(ValidationError::constant_object(
                Location::clone(here),
                crate::paths::capture_evaluation_path(tracker, here),
                location.into(),
                instance,
                &self.value,
            )),
            (true, _) => Ok(()),
        }
    }

    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        [instance]
            .into_iter()
            .filter_map(Value::as_object)
            .any(|item| cmp::equal_objects(&self.value, item))
    }
}

pub(crate) struct ConstStringValidator {
    value: String,
    location: Location,
}

impl ConstStringValidator {
    #[inline]
    pub(crate) fn compile(value: &str, location: Location) -> CompilationResult<'_> {
        Ok(Box::new(ConstStringValidator {
            value: value.to_string(),
            location,
        }))
    }
}

impl Validate for ConstStringValidator {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let rejected = !self.is_valid(instance, ctx);
        let failure = rejected.then(|| {
            ValidationError::constant_string(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                &self.value,
            )
        });
        failure.map_or(Ok(()), Err)
    }

    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        let Some(item) = instance.as_str() else {
            return false;
        };
        let (lhs, rhs) = (self.value.as_bytes(), item.as_bytes());
        if lhs.len() != rhs.len() {
            return false;
        }
        congruent(lhs, rhs)
    }
}

#[inline]
pub(crate) fn compile<'a>(
    ctx: &compiler::Context,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    let location = ctx.location().join("const");
    let compiled = if let Value::String(string) = schema {
        ConstStringValidator::compile(string, location)
    } else if let Value::Object(map) = schema {
        ConstObjectValidator::compile(map, location)
    } else {
        match schema {
            Value::Number(item) => ConstNumberValidator::compile(item, location),
            Value::Bool(item) => ConstBooleanValidator::compile(*item, location),
            Value::Array(items) => ConstArrayValidator::compile(items, location),
            Value::Null | Value::String(_) | Value::Object(_) => {
                ConstNullValidator::compile(location)
            }
        }
    };
    Some(compiled)
}

fn congruent(lhs: &[u8], rhs: &[u8]) -> bool {
    match (lhs, rhs) {
        ([], []) => true,
        ([left], [right]) => (left ^ right) == 0,
        _ if lhs.len() != rhs.len() => false,
        _ => {
            let pivot = lhs.len() >> 1;
            let (lhs_front, lhs_back) = lhs.split_at(pivot);
            let (rhs_front, rhs_back) = rhs.split_at(pivot);
            congruent(lhs_front, rhs_front) && congruent(lhs_back, rhs_back)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"const": 1}), &json!(2), "/const")]
    #[test_case(&json!({"const": null}), &json!(3), "/const")]
    #[test_case(&json!({"const": false}), &json!(4), "/const")]
    #[test_case(&json!({"const": []}), &json!(5), "/const")]
    #[test_case(&json!({"const": {}}), &json!(6), "/const")]
    #[test_case(&json!({"const": ""}), &json!(7), "/const")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }

    // Tests for arbitrary-precision const validation
    #[cfg(feature = "arbitrary-precision")]
    mod arbitrary_precision {
        use crate::tests_util;
        use serde_json::Value;
        use test_case::test_case;

        fn parse_json(json: &str) -> Value {
            serde_json::from_str(json).unwrap()
        }

        #[test_case(r#"{"const": 18446744073709551617}"#, "18446744073709551617", true; "large int exact match")]
        #[test_case(r#"{"const": 18446744073709551617}"#, "18446744073709551616", false; "large int different by one")]
        #[test_case(r#"{"const": 18446744073709551617}"#, "18446744073709551618", false; "large int different by one above")]
        #[test_case(r#"{"const": -9223372036854775809}"#, "-9223372036854775809", true; "large negative int match")]
        #[test_case(r#"{"const": -9223372036854775809}"#, "-9223372036854775808", false; "large negative int different")]
        #[test_case(r#"{"const": 0.1}"#, "0.1", true; "decimal exact match")]
        #[test_case(r#"{"const": 0.1}"#, "0.10000000000000001", false; "decimal precision difference")]
        fn const_arbitrary_precision(schema_json: &str, instance_json: &str, expected_valid: bool) {
            let schema = parse_json(schema_json);
            let instance = parse_json(instance_json);
            if expected_valid {
                tests_util::is_valid(&schema, &instance);
            } else {
                tests_util::is_not_valid(&schema, &instance);
            }
        }
    }
}
