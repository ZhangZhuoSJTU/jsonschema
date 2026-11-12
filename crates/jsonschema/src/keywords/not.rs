use crate::{
    compiler,
    error::ValidationError,
    keywords::CompilationResult,
    node::SchemaNode,
    paths::{LazyLocation, RefTracker},
    validator::{Validate, ValidationContext},
};
use serde_json::{Map, Value};

pub(crate) struct NotValidator {
    // needed only for error representation
    original: Value,
    node: SchemaNode,
}

impl NotValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context, schema: &'a Value) -> CompilationResult<'a> {
        let ctx = ctx.new_at_location("not");
        Ok(Box::new(NotValidator {
            original: schema.clone(),
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
        }))
    }
}

#[inline]
pub(crate) fn compile<'a>(
    ctx: &compiler::Context,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    Some(NotValidator::compile(ctx, schema))
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(
            &json!({"not": {"type": "string"}}),
            &json!("foo"),
            "/not",
        );
    }
}

mod validation;
