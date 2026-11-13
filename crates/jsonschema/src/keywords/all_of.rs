use crate::{
    compiler,
    error::{ErrorIterator, ValidationError},
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    types::JsonType,
    validator::{EvaluationResult, Validate, ValidationContext},
};
use serde_json::{Map, Value};

use super::CompilationResult;

pub(crate) struct AllOfValidator {
    schemas: Vec<SchemaNode>,
}

impl AllOfValidator {
    #[inline]
    pub(crate) fn compile<'a>(
        ctx: &compiler::Context,
        items: &'a [Value],
    ) -> CompilationResult<'a> {
        let ctx = ctx.new_at_location("allOf");
        let schemas = items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let ctx = ctx.new_at_location(idx);
                compiler::compile(&ctx, ctx.as_resource_ref(item))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Box::new(AllOfValidator { schemas }))
    }
}

impl Validate for AllOfValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        unanimous(&self.schemas, instance, ctx)
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        self.schemas
            .iter()
            .try_for_each(|schema| schema.validate(instance, location, tracker, ctx))
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        let mut errors = Vec::new();
        for node in &self.schemas {
            errors.extend(node.iter_errors(instance, location, tracker, ctx));
        }
        ErrorIterator::from_iterator(errors.into_iter())
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        let children = self
            .schemas
            .iter()
            .map(|node| node.evaluate_instance(instance, location, tracker, ctx))
            .collect();
        EvaluationResult::from_children(children)
    }
}

pub(crate) struct SingleValueAllOfValidator {
    node: SchemaNode,
}

impl SingleValueAllOfValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context, schema: &'a Value) -> CompilationResult<'a> {
        let all_of_ctx = ctx.new_at_location("allOf");
        let item_ctx = all_of_ctx.new_at_location(0);
        let node = compiler::compile(&item_ctx, item_ctx.as_resource_ref(schema))?;
        Ok(Box::new(SingleValueAllOfValidator { node }))
    }
}

impl Validate for SingleValueAllOfValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        self.node.is_valid(instance, ctx)
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        self.node.validate(instance, location, tracker, ctx)
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        self.node.iter_errors(instance, location, tracker, ctx)
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        EvaluationResult::from(
            self.node
                .evaluate_instance(instance, location, tracker, ctx),
        )
    }
}

#[inline]
pub(crate) fn compile<'a>(
    ctx: &compiler::Context,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    let Value::Array(items) = schema else {
        let location = ctx.location().join("allOf");
        return Some(Err(ValidationError::single_type_error(
            location.clone(),
            location,
            Location::new(),
            schema,
            JsonType::Array,
        )));
    };
    Some(match items.as_slice() {
        // A lone subschema does not need the generic machinery
        [single] => SingleValueAllOfValidator::compile(ctx, single),
        _ => AllOfValidator::compile(ctx, items),
    })
}

fn unanimous(nodes: &[SchemaNode], instance: &Value, ctx: &mut ValidationContext) -> bool {
    match nodes {
        [] => true,
        [only] => only.is_valid(instance, ctx),
        _ => {
            let (front, back) = nodes.split_at(nodes.len() >> 1);
            !(!unanimous(front, instance, ctx) || !unanimous(back, instance, ctx))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"allOf": [{"type": "string"}]}), &json!(1), "/allOf/0/type")]
    #[test_case(&json!({"allOf": [{"type": "integer"}, {"maximum": 5}]}), &json!(6), "/allOf/1/maximum")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }
}
