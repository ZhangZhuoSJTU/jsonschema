use crate::{
    compiler,
    error::ValidationError,
    evaluation::ErrorDescription,
    keywords::CompilationResult,
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    types::JsonType,
    validator::{EvaluationResult, Validate, ValidationContext},
};
use serde_json::{Map, Value};

pub(crate) struct OneOfValidator {
    schemas: Vec<SchemaNode>,
    location: Location,
}

impl OneOfValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context, schema: &'a Value) -> CompilationResult<'a> {
        let Value::Array(items) = schema else {
            let location = ctx.location().join("oneOf");
            return Err(ValidationError::single_type_error(
                location.clone(),
                location,
                Location::new(),
                schema,
                JsonType::Array,
            ));
        };
        let ctx = ctx.new_at_location("oneOf");
        let schemas = items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let ctx = ctx.new_at_location(idx);
                compiler::compile(&ctx, ctx.as_resource_ref(item))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let location = ctx.location().clone();
        Ok(Box::new(OneOfValidator { schemas, location }))
    }

    fn get_first_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> Option<usize> {
        locate(&self.schemas, 0, instance, ctx)
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn are_others_valid(&self, instance: &Value, idx: usize, ctx: &mut ValidationContext) -> bool {
        let (_, rest) = self.schemas.split_at((idx + 1).min(self.schemas.len()));
        locate(rest, idx, instance, ctx).map_or(false, |found| found >= idx)
    }
}

/// Optimized validator for `oneOf` with a single subschema.
/// With exactly one schema, `oneOf` behaves identically to `anyOf`.
pub(crate) struct SingleOneOfValidator {
    node: SchemaNode,
    location: Location,
}

impl SingleOneOfValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context, schema: &'a Value) -> CompilationResult<'a> {
        let one_of_ctx = ctx.new_at_location("oneOf");
        let item_ctx = one_of_ctx.new_at_location(0);
        let node = compiler::compile(&item_ctx, item_ctx.as_resource_ref(schema))?;
        Ok(Box::new(SingleOneOfValidator {
            node,
            location: one_of_ctx.location().clone(),
        }))
    }
}

impl Validate for SingleOneOfValidator {
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
            Err(ValidationError::one_of_not_valid(
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

impl Validate for OneOfValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        match self.get_first_valid(instance, ctx) {
            Some(idx) => !self.are_others_valid(instance, idx, ctx),
            None => false,
        }
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let first_valid_idx = self.get_first_valid(instance, ctx);
        if let Some(idx) = first_valid_idx {
            if self.are_others_valid(instance, idx, ctx) {
                return Err(ValidationError::one_of_multiple_valid(
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
                ));
            }
            Ok(())
        } else {
            Err(ValidationError::one_of_not_valid(
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
        // Use cheap `is_valid` first, then run full `evaluate` only on matching schemas.
        let first_valid_idx = self.get_first_valid(instance, ctx);

        let Some(first_idx) = first_valid_idx else {
            let failures: Vec<_> = self
                .schemas
                .iter()
                .map(|node| node.evaluate_instance(instance, location, tracker, ctx))
                .collect();
            return EvaluationResult::Invalid {
                errors: Vec::new(),
                children: failures,
                annotations: None,
            };
        };

        if self.are_others_valid(instance, first_idx, ctx) {
            let mut successes = Vec::new();
            for (idx, node) in self.schemas.iter().enumerate() {
                if idx != first_idx && !node.is_valid(instance, ctx) {
                    continue;
                }
                let child = node.evaluate_instance(instance, location, tracker, ctx);
                if child.valid {
                    successes.push(child);
                }
            }
            EvaluationResult::Invalid {
                errors: vec![ErrorDescription::new(
                    "oneOf",
                    "more than one subschema succeeded".to_string(),
                )],
                children: successes,
                annotations: None,
            }
        } else {
            let child = self.schemas[first_idx].evaluate_instance(instance, location, tracker, ctx);
            EvaluationResult::from(child)
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
        Value::Array(items) => match items.as_slice() {
            [item] => Some(SingleOneOfValidator::compile(ctx, item)),
            _ => Some(OneOfValidator::compile(ctx, schema)),
        },
        _ => Some(OneOfValidator::compile(ctx, schema)),
    }
}

fn locate(
    nodes: &[SchemaNode],
    base: usize,
    instance: &Value,
    ctx: &mut ValidationContext,
) -> Option<usize> {
    match nodes {
        [] => None,
        [only] => Some(base).filter(|_| only.is_valid(instance, ctx)),
        _ => {
            let pivot = nodes.len() >> 1;
            let (front, back) = nodes.split_at(pivot);
            match locate(front, base, instance, ctx) {
                None => locate(back, base + pivot, instance, ctx),
                found => found,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::{json, Value};
    use test_case::test_case;

    #[test_case(&json!({"oneOf": [{"type": "string"}]}), &json!(0), "/oneOf")]
    #[test_case(&json!({"oneOf": [{"type": "string"}, {"maxLength": 3}]}), &json!(""), "/oneOf")]
    fn location(schema: &Value, instance: &Value, expected: &str) {
        tests_util::assert_schema_location(schema, instance, expected);
    }
}
