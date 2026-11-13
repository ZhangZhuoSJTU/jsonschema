use crate::{
    compiler,
    error::{no_error, ErrorIterator, ValidationError},
    keywords::CompilationResult,
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    validator::{EvaluationResult, Validate, ValidationContext},
};
use serde_json::{Map, Value};

pub(crate) struct PropertyNamesObjectValidator {
    node: SchemaNode,
}

impl PropertyNamesObjectValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context, schema: &'a Value) -> CompilationResult<'a> {
        let ctx = ctx.new_at_location("propertyNames");
        Ok(Box::new(PropertyNamesObjectValidator {
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
        }))
    }
}

impl Validate for PropertyNamesObjectValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        let Value::Object(item) = instance else {
            return true;
        };
        for key in item.keys() {
            let wrapper = Value::String(key.clone());
            if !self.node.is_valid(&wrapper, ctx) {
                return false;
            }
        }
        true
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Value::Object(item) = &instance {
            for key in item.keys() {
                let wrapper = Value::String(key.clone());
                if let Err(error) = self.node.validate(&wrapper, location, tracker, ctx) {
                    let schema_path = error.schema_path().clone();
                    let evaluation_path =
                        crate::paths::capture_evaluation_path(tracker, &schema_path);
                    return Err(ValidationError::property_names(
                        schema_path,
                        evaluation_path,
                        location.into(),
                        instance,
                        error.to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        if let Value::Object(item) = &instance {
            let mut errors = Vec::new();
            for key in item.keys() {
                let wrapper = Value::String(key.clone());
                for error in self.node.iter_errors(&wrapper, location, tracker, ctx) {
                    let schema_path = error.schema_path().clone();
                    errors.push(ValidationError::property_names(
                        schema_path.clone(),
                        crate::paths::capture_evaluation_path(tracker, &schema_path),
                        location.into(),
                        instance,
                        error.to_owned(),
                    ));
                }
            }
            ErrorIterator::from_iterator(errors.into_iter())
        } else {
            no_error()
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        let Value::Object(item) = instance else {
            return EvaluationResult::valid_empty();
        };
        let children = item
            .keys()
            .map(|key| {
                let wrapper = Value::String(key.clone());
                self.node
                    .evaluate_instance(&wrapper, location, tracker, ctx)
            })
            .collect();
        EvaluationResult::from_children(children)
    }
}

pub(crate) struct PropertyNamesBooleanValidator {
    location: Location,
}

impl PropertyNamesBooleanValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context) -> CompilationResult<'a> {
        let location = ctx.location().join("propertyNames");
        Ok(Box::new(PropertyNamesBooleanValidator { location }))
    }
}

impl Validate for PropertyNamesBooleanValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        // A `false` schema rejects every name, so only empty objects pass
        match instance {
            Value::Object(item) => item.is_empty(),
            _ => true,
        }
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if self.is_valid(instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::false_schema(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
            ))
        }
    }
}

#[inline]
pub(crate) fn compile<'a>(
    ctx: &compiler::Context,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    match schema {
        Value::Object(_) => Some(PropertyNamesObjectValidator::compile(ctx, schema)),
        Value::Bool(false) => Some(PropertyNamesBooleanValidator::compile(ctx)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"propertyNames": false}), &json!({"foo": 1}), "/propertyNames")]
    #[test_case(&json!({"propertyNames": {"minLength": 2}}), &json!({"f": 1}), "/propertyNames/minLength")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }
}
