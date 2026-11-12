use crate::{
    compiler,
    error::ValidationError,
    evaluation::{Annotations, ErrorDescription},
    keywords::CompilationResult,
    node::SchemaNode,
    paths::{LazyLocation, RefTracker},
    validator::{EvaluationResult, Validate, ValidationContext},
    Draft,
};
use serde_json::{Map, Value};

use super::helpers::map_get_u64;

pub(crate) struct ContainsValidator {
    node: SchemaNode,
}

impl ContainsValidator {
    #[inline]
    pub(crate) fn compile<'a>(ctx: &compiler::Context, schema: &'a Value) -> CompilationResult<'a> {
        let ctx = ctx.new_at_location("contains");
        Ok(Box::new(ContainsValidator {
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
        }))
    }
}

/// `minContains` validation. Used only if there is no `maxContains` present.
///
/// Docs: <https://json-schema.org/draft/2019-09/json-schema-validation.html#rfc.section.6.4.5>
pub(crate) struct MinContainsValidator {
    node: SchemaNode,
    min_contains: u64,
}

impl MinContainsValidator {
    #[inline]
    pub(crate) fn compile<'a>(
        ctx: &compiler::Context,
        schema: &'a Value,
        min_contains: u64,
    ) -> CompilationResult<'a> {
        let ctx = ctx.new_at_location("minContains");
        Ok(Box::new(MinContainsValidator {
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
            min_contains,
        }))
    }
}

/// `maxContains` validation. Used only if there is no `minContains` present.
///
/// Docs: <https://json-schema.org/draft/2019-09/json-schema-validation.html#rfc.section.6.4.4>
pub(crate) struct MaxContainsValidator {
    node: SchemaNode,
    max_contains: u64,
}

impl MaxContainsValidator {
    #[inline]
    pub(crate) fn compile<'a>(
        ctx: &compiler::Context,
        schema: &'a Value,
        max_contains: u64,
    ) -> CompilationResult<'a> {
        let ctx = ctx.new_at_location("maxContains");
        Ok(Box::new(MaxContainsValidator {
            node: compiler::compile(&ctx, ctx.as_resource_ref(schema))?,
            max_contains,
        }))
    }
}

/// `maxContains` & `minContains` validation combined.
///
/// Docs:
///   `maxContains` - <https://json-schema.org/draft/2019-09/json-schema-validation.html#rfc.section.6.4.4>
///   `minContains` - <https://json-schema.org/draft/2019-09/json-schema-validation.html#rfc.section.6.4.5>
pub(crate) struct MinMaxContainsValidator {
    node: SchemaNode,
    min_contains: u64,
    max_contains: u64,
}

impl MinMaxContainsValidator {
    #[inline]
    pub(crate) fn compile<'a>(
        ctx: &compiler::Context,
        schema: &'a Value,
        min_contains: u64,
        max_contains: u64,
    ) -> CompilationResult<'a> {
        Ok(Box::new(MinMaxContainsValidator {
            node: compiler::compile(ctx, ctx.as_resource_ref(schema))?,
            min_contains,
            max_contains,
        }))
    }
}

#[inline]
pub(crate) fn compile<'a>(
    ctx: &compiler::Context,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    match ctx.draft() {
        Draft::Draft4 | Draft::Draft6 | Draft::Draft7 => {
            Some(ContainsValidator::compile(ctx, schema))
        }
        Draft::Draft201909 | Draft::Draft202012 => compile_contains(ctx, parent, schema),
        _ => None,
    }
}

#[inline]
fn compile_contains<'a>(
    ctx: &compiler::Context,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    let min_contains = match map_get_u64(parent, ctx, "minContains").transpose() {
        Ok(n) => n,
        Err(err) => return Some(Err(err)),
    };
    let max_contains = match map_get_u64(parent, ctx, "maxContains").transpose() {
        Ok(n) => n,
        Err(err) => return Some(Err(err)),
    };

    match (min_contains, max_contains) {
        (Some(min), Some(max)) => Some(MinMaxContainsValidator::compile(ctx, schema, min, max)),
        (Some(min), None) => Some(MinContainsValidator::compile(ctx, schema, min)),
        (None, Some(max)) => Some(MaxContainsValidator::compile(ctx, schema, max)),
        (None, None) => Some(ContainsValidator::compile(ctx, schema)),
    }
}

#[cfg(test)]
mod tests {
    use crate::tests_util;
    use serde_json::json;

    #[test]
    fn location() {
        tests_util::assert_schema_location(
            &json!({"contains": {"const": 2}}),
            &json!([]),
            "/contains",
        );
    }
}

mod contains_validation;
mod limits_validation;
