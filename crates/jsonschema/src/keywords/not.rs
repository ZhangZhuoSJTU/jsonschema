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
        let node = compiler::compile(&ctx, ctx.as_resource_ref(schema))?;
        let original = schema.clone();
        Ok(Box::new(NotValidator { original, node }))
    }
}

impl Validate for NotValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        match self.node.is_valid(instance, ctx) {
            true => false,
            false => true,
        }
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        // `not` succeeds exactly when the wrapped schema rejects the instance
        if self.node.is_valid(instance, ctx) {
            let schema_location = self.node.location();
            return Err(ValidationError::not(
                schema_location.clone(),
                crate::paths::capture_evaluation_path(tracker, schema_location),
                location.into(),
                instance,
                self.original.clone(),
            ));
        }
        Ok(())
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
