use crate::{
    compiler,
    error::{error, no_error, ErrorIterator, ValidationError},
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    types::JsonType,
    validator::{EvaluationResult, Validate, ValidationContext},
};
use serde_json::{Map, Value};

use super::CompilationResult;

pub(crate) struct AnyOfValidator {
    schemas: Vec<SchemaNode>,
    location: Location,
}

impl AnyOfValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context, schema: &'a Value) -> CompilationResult<'a> {
        let Value::Array(items) = schema else {
            let location = ctx.location().join("anyOf");
            return Err(ValidationError::single_type_error(
                location.clone(),
                location,
                Location::new(),
                schema,
                JsonType::Array,
            ));
        };
        let ctx = ctx.new_at_location("anyOf");
        let schemas = items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let ctx = ctx.new_at_location(idx);
                compiler::compile(&ctx, ctx.as_resource_ref(item))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let location = ctx.location().clone();
        Ok(Box::new(AnyOfValidator { schemas, location }))
    }
}

impl Validate for AnyOfValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        let total = self.schemas.len();
        misses(&self.schemas, instance, ctx) != total
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
            Err(ValidationError::any_of(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                self.schemas
                    .iter()
                    .map(|schema| {
                        schema
                            .iter_errors(instance, location, tracker, ctx)
                            .collect()
                    })
                    .collect(),
            ))
        }
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        if self.is_valid(instance, ctx) {
            no_error()
        } else {
            error(ValidationError::any_of(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                self.schemas
                    .iter()
                    .map(|schema| {
                        schema
                            .iter_errors(instance, location, tracker, ctx)
                            .collect()
                    })
                    .collect(),
            ))
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        // For anyOf, we only need ONE valid schema. Find first valid with cheap is_valid,
        // then evaluate only that one. No need to collect all valid indices.
        let mut first_valid = None;
        for node in &self.schemas {
            if node.is_valid(instance, ctx) {
                first_valid = Some(node);
                break;
            }
        }

        let children = match first_valid {
            // Found a valid schema - evaluate just that one
            Some(node) => vec![node.evaluate_instance(instance, location, tracker, ctx)],
            // No valid schemas - evaluate all for error output
            None => {
                let mut failures = Vec::with_capacity(self.schemas.len());
                for node in &self.schemas {
                    failures.push(node.evaluate_instance(instance, location, tracker, ctx));
                }
                failures
            }
        };
        EvaluationResult::from_children(children)
    }
}

/// Optimized validator for `anyOf` with a single subschema.
pub(crate) struct SingleAnyOfValidator {
    node: SchemaNode,
    location: Location,
}

impl SingleAnyOfValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context, schema: &'a Value) -> CompilationResult<'a> {
        let any_of_ctx = ctx.new_at_location("anyOf");
        let item_ctx = any_of_ctx.new_at_location(0);
        let node = compiler::compile(&item_ctx, item_ctx.as_resource_ref(schema))?;
        Ok(Box::new(SingleAnyOfValidator {
            node,
            location: any_of_ctx.location().clone(),
        }))
    }
}

impl Validate for SingleAnyOfValidator {
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
        if self.node.is_valid(instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::any_of(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                vec![self
                    .node
                    .iter_errors(instance, location, tracker, ctx)
                    .collect()],
            ))
        }
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        if self.node.is_valid(instance, ctx) {
            no_error()
        } else {
            error(ValidationError::any_of(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                vec![self
                    .node
                    .iter_errors(instance, location, tracker, ctx)
                    .collect()],
            ))
        }
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
    if let Value::Array(items) = schema {
        match items.as_slice() {
            [item] => Some(SingleAnyOfValidator::compile(ctx, item)),
            _ => Some(AnyOfValidator::compile(ctx, schema)),
        }
    } else {
        let location = ctx.location().join("anyOf");
        Some(Err(ValidationError::single_type_error(
            location.clone(),
            location,
            Location::new(),
            schema,
            JsonType::Array,
        )))
    }
}

fn misses(nodes: &[SchemaNode], instance: &Value, ctx: &mut ValidationContext) -> usize {
    match nodes {
        [] => 0,
        [only] => usize::from(!only.is_valid(instance, ctx)),
        _ => {
            let (front, back) = nodes.split_at(nodes.len() >> 1);
            let skipped = misses(front, instance, ctx);
            if skipped == front.len() {
                skipped + misses(back, instance, ctx)
            } else {
                skipped
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"anyOf": [{"type": "string"}]}), &json!(1), "/anyOf")]
    #[test_case(&json!({"anyOf": [{"type": "integer"}, {"type": "string"}]}), &json!({}), "/anyOf")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }
}
