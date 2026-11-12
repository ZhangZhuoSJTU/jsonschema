use crate::{
    compiler,
    error::{no_error, ErrorIterator, ValidationError},
    keywords::{boolean::FalseValidator, CompilationResult},
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    types::{JsonType, JsonTypeSet},
    validator::{Validate, ValidationContext},
};
use serde_json::{Map, Value};

pub(crate) struct AdditionalItemsObjectValidator {
    node: SchemaNode,
    items_count: usize,
}
impl AdditionalItemsObjectValidator {
    #[inline]
    pub(crate) fn compile<'a>(
        ctx: &compiler::Context,
        schema: &'a Value,
        items_count: usize,
    ) -> CompilationResult<'a> {
        let node = compiler::compile(ctx, ctx.as_resource_ref(schema))?;
        Ok(Box::new(AdditionalItemsObjectValidator {
            node,
            items_count,
        }))
    }
}

pub(crate) struct AdditionalItemsBooleanValidator {
    items_count: usize,
    location: Location,
}
impl AdditionalItemsBooleanValidator {
    #[inline]
    pub(crate) fn compile<'a>(items_count: usize, location: Location) -> CompilationResult<'a> {
        Ok(Box::new(AdditionalItemsBooleanValidator {
            items_count,
            location,
        }))
    }
}

#[inline]
pub(crate) fn compile<'a>(
    ctx: &compiler::Context,
    parent: &Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    if let Some(items) = parent.get("items") {
        match items {
            Value::Object(_) => None,
            Value::Array(items) => {
                let kctx = ctx.new_at_location("additionalItems");
                let items_count = items.len();
                match schema {
                    Value::Object(_) => Some(AdditionalItemsObjectValidator::compile(
                        &kctx,
                        schema,
                        items_count,
                    )),
                    Value::Bool(false) => Some(AdditionalItemsBooleanValidator::compile(
                        items_count,
                        kctx.location().clone(),
                    )),
                    _ => None,
                }
            }
            Value::Bool(value) => {
                if *value {
                    None
                } else {
                    let location = ctx.location().join("additionalItems");
                    Some(FalseValidator::compile(location))
                }
            }
            _ => {
                let location = ctx.location().join("additionalItems");
                Some(Err(ValidationError::multiple_type_error(
                    location.clone(),
                    location,
                    Location::new(),
                    schema,
                    JsonTypeSet::empty()
                        .insert(JsonType::Object)
                        .insert(JsonType::Array)
                        .insert(JsonType::Boolean),
                )))
            }
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use referencing::Draft;
    use serde_json::{json, Value};
    use test_case::test_case;

    // When items: false and additionalItems: false, items runs first (lower priority)
    #[test_case(&json!({"additionalItems": false, "items": false}), &json!([1]), "/items")]
    #[test_case(&json!({"additionalItems": false, "items": [{}]}), &json!([1, 2]), "/additionalItems")]
    #[test_case(&json!({"additionalItems": {"type": "string"}, "items": [{}]}), &json!([1, 2]), "/additionalItems/type")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        let validator = crate::options()
            .with_draft(Draft::Draft7)
            .build(schema)
            .expect("Invalid schema");
        let error = validator.validate(instance).expect_err("Should fail");
        assert_eq!(error.schema_path().as_str(), expected);
    }
}

mod validation;
