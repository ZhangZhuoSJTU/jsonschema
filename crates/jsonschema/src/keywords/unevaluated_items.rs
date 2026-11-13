//! Implementation of the `unevaluatedItems` keyword.
//!
//! This keyword validates array items that were not evaluated by other keywords like
//! `items`, `prefixItems`, `contains`, or nested schemas in combinators (`allOf`, `anyOf`, `oneOf`),
//! conditionals, and references.
//!
//! The implementation eagerly compiles a recursive `ItemsValidators` structure during
//! schema compilation, using `Arc<OnceLock>` for circular reference handling.
use referencing::Draft;
use serde_json::{Map, Value};
use std::sync::{Arc, OnceLock};

use crate::{
    compiler,
    evaluation::ErrorDescription,
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    validator::{EvaluationResult, Validate, ValidationContext},
    ValidationError,
};

use super::CompilationResult;

/// Lazy items validators that are compiled on first access.
/// Used for $recursiveRef and circular references to handle cycles during compilation.
pub(crate) type PendingItemsValidators = Arc<OnceLock<ItemsValidators>>;

/// Holds compiled validators for items evaluation in unevaluatedItems.
/// This structure is built during schema compilation and used during validation.
#[derive(Debug, Clone)]
pub(crate) struct ItemsValidators {
    /// Validator from "unevaluatedItems" keyword itself
    unevaluated: Option<SchemaNode>,
    /// Validator from "contains" keyword
    contains: Option<SchemaNode>,
    /// Reference validators from "$ref" keyword
    ref_: Option<RefValidator>,
    /// Reference validators from "$dynamicRef" keyword (Draft 2020-12+)
    dynamic_ref: Option<Box<ItemsValidators>>,
    /// Validators from "$recursiveRef" keyword (Draft 2019-09 only)
    recursive_ref: Option<PendingItemsValidators>,
    /// Items limit - for Draft 2019-09 "items" keyword behavior
    /// If present, marks first N items as evaluated
    items_limit: Option<usize>,
    /// Items schema present - for Draft 2020-12+ "items" keyword
    /// If true, marks ALL items as evaluated
    items_all: bool,
    /// Prefix items count - from "prefixItems" keyword
    prefix_items: Option<usize>,
    /// Conditional validators from "if/then/else" keywords
    conditional: Option<Box<ConditionalValidators>>,
    /// Validators from "allOf" keyword
    all_of: Option<Vec<(SchemaNode, ItemsValidators)>>,
    /// Validators from "anyOf" keyword
    any_of: Option<Vec<(SchemaNode, ItemsValidators)>>,
    /// Validators from "oneOf" keyword
    one_of: Option<Vec<(SchemaNode, ItemsValidators)>>,
}

/// Reference validator - wraps `ItemsValidators`
#[derive(Debug, Clone)]
struct RefValidator(Box<ItemsValidators>);

/// Conditional validators from "if/then/else" keywords
#[derive(Debug, Clone)]
struct ConditionalValidators {
    condition: SchemaNode,
    if_: ItemsValidators,
    then_: Option<ItemsValidators>,
    else_: Option<ItemsValidators>,
}

impl ItemsValidators {
    /// Mark all items that are evaluated by this schema.
    fn mark_evaluated_indexes(
        &self,
        instance: &Value,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
    ) {
        // Draft 2020-12+ `items` applies to every element, so the whole array is
        // evaluated up front and no other keyword can contribute any marks.
        if self.items_all {
            indexes.fill(true);
            return;
        }

        // Resolve references first: a `$ref` target can mark items anywhere in
        // the array, including positions the adjacent keywords never reach.
        self.mark_referenced(instance, indexes, ctx);

        // Adjacent keywords (`items`/`prefixItems`) only ever mark a leading run.
        self.mark_adjacent(indexes);

        // If the whole array is already accounted for, skip the remaining passes.
        if indexes.iter().all(|&evaluated| evaluated) {
            return;
        }

        // Elementwise keywords inspect each remaining item in turn.
        self.mark_elementwise(instance, indexes, ctx);

        // Applicators contribute marks from their nested schemas.
        self.mark_applicators(instance, indexes, ctx);
    }

    /// Recurse into `$ref`, `$recursiveRef` and `$dynamicRef` targets so that
    /// whatever they mark is propagated back to this schema's view.
    fn mark_referenced(
        &self,
        instance: &Value,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
    ) {
        if let Some(ref_) = &self.ref_ {
            ref_.0.mark_evaluated_indexes(instance, indexes, ctx);
        }

        if let Some(recursive_ref) = &self.recursive_ref {
            if let Some(validators) = recursive_ref.get() {
                validators.mark_evaluated_indexes(instance, indexes, ctx);
            }
        }

        if let Some(dynamic_ref) = &self.dynamic_ref {
            dynamic_ref.mark_evaluated_indexes(instance, indexes, ctx);
        }
    }

    /// Mark the leading run of items evaluated by `items` and `prefixItems`.
    fn mark_adjacent(&self, indexes: &mut Vec<bool>) {
        if let Some(limit) = self.items_limit {
            // Draft 2019-09: `items` (tuple form) marks the first `limit` items.
            // `usize::MAX` denotes the object form, which applies to every item.
            if limit == usize::MAX {
                indexes.fill(true);
            } else {
                indexes[..limit].fill(true);
            }
        }

        if let Some(limit) = self.prefix_items {
            // prefixItems marks first N items
            for evaluated in indexes.iter_mut().take(limit) {
                *evaluated = true;
            }
        }
    }

    /// Mark items that match `contains` or `unevaluatedItems`.
    fn mark_elementwise(
        &self,
        instance: &Value,
        indexes: &mut [bool],
        ctx: &mut ValidationContext,
    ) {
        if let Value::Array(items) = instance {
            for (item, evaluated) in items.iter().zip(indexes.iter_mut()) {
                if *evaluated {
                    continue;
                }
                // contains marks items that match
                if let Some(validator) = &self.contains {
                    if validator.is_valid(item, ctx) {
                        *evaluated = true;
                        continue;
                    }
                }
                // unevaluatedItems itself can mark items
                if let Some(validator) = &self.unevaluated {
                    if validator.is_valid(item, ctx) {
                        *evaluated = true;
                    }
                }
            }
        }
    }

    /// Recurse into conditionals and combinators, letting successful branches
    /// contribute their own marks.
    fn mark_applicators(
        &self,
        instance: &Value,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
    ) {
        if let Some(conditional) = &self.conditional {
            conditional.mark_evaluated_indexes(instance, indexes, ctx);
        }

        if let Some(all_of) = &self.all_of {
            for (node, validators) in all_of {
                if node.is_valid(instance, ctx) {
                    validators.mark_evaluated_indexes(instance, indexes, ctx);
                }
            }
        }

        if let Some(any_of) = &self.any_of {
            for (node, validators) in any_of {
                if node.is_valid(instance, ctx) {
                    validators.mark_evaluated_indexes(instance, indexes, ctx);
                }
            }
        }

        if let Some(one_of) = &self.one_of {
            let mut matched = None;
            let mut matches = 0usize;
            for (node, validators) in one_of {
                if node.is_valid(instance, ctx) {
                    matches += 1;
                    if matches > 1 {
                        // More than one match: no single branch may contribute.
                        matched = None;
                        break;
                    }
                    matched = Some(validators);
                }
            }
            if let Some(validators) = matched {
                validators.mark_evaluated_indexes(instance, indexes, ctx);
            }
        }
    }
}

impl ConditionalValidators {
    fn mark_evaluated_indexes(
        &self,
        instance: &Value,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
    ) {
        if self.condition.is_valid(instance, ctx) {
            self.if_.mark_evaluated_indexes(instance, indexes, ctx);
            if let Some(then_) = &self.then_ {
                then_.mark_evaluated_indexes(instance, indexes, ctx);
            }
        } else if let Some(else_) = &self.else_ {
            else_.mark_evaluated_indexes(instance, indexes, ctx);
        }
    }
}

/// Compile all items validators for a schema.
///
/// Recursively builds the `ItemsValidators` tree by examining all keywords that
/// can evaluate items. Handles circular references via pending nodes cached
/// by location and schema pointer.
fn compile_items_validators<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<ItemsValidators, ValidationError<'a>> {
    let unevaluated = compile_unevaluated(ctx, parent)?;
    let contains = compile_contains(ctx, parent)?;
    let ref_ = compile_ref(ctx, parent)?;
    let dynamic_ref = compile_dynamic_ref(ctx, parent)?;
    let recursive_ref = compile_recursive_ref(ctx, parent)?;

    // Determine items behavior based on draft
    let (items_limit, items_all) = compile_items(ctx, parent)?;
    let prefix_items = compile_prefix_items(ctx, parent)?;

    let conditional = compile_conditional(ctx, parent)?;
    let all_of = compile_all_of(ctx, parent)?;
    let any_of = compile_any_of(ctx, parent)?;
    let one_of = compile_one_of(ctx, parent)?;

    Ok(ItemsValidators {
        unevaluated,
        contains,
        ref_,
        dynamic_ref,
        recursive_ref,
        items_limit,
        items_all,
        prefix_items,
        conditional,
        all_of,
        any_of,
        one_of,
    })
}

fn compile_unevaluated<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<SchemaNode>, ValidationError<'a>> {
    if let Some(subschema) = parent.get("unevaluatedItems") {
        let unevaluated_ctx = ctx.new_at_location("unevaluatedItems");
        Ok(Some(
            compiler::compile(&unevaluated_ctx, unevaluated_ctx.as_resource_ref(subschema))
                .map_err(ValidationError::to_owned)?,
        ))
    } else {
        Ok(None)
    }
}

fn compile_contains<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<SchemaNode>, ValidationError<'a>> {
    if let Some(subschema) = parent.get("contains") {
        let contains_ctx = ctx.new_at_location("contains");
        Ok(Some(
            compiler::compile(&contains_ctx, contains_ctx.as_resource_ref(subschema))
                .map_err(ValidationError::to_owned)?,
        ))
    } else {
        Ok(None)
    }
}

fn compile_ref<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<RefValidator>, ValidationError<'a>> {
    if let Some(Value::String(reference)) = parent.get("$ref") {
        let resolved = ctx.lookup(reference)?;
        if let Value::Object(subschema) = resolved.contents() {
            let validators =
                compile_items_validators(ctx, subschema).map_err(ValidationError::to_owned)?;
            return Ok(Some(RefValidator(Box::new(validators))));
        }
    }
    Ok(None)
}

fn compile_dynamic_ref<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<Box<ItemsValidators>>, ValidationError<'a>> {
    if let Some(Value::String(reference)) = parent.get("$dynamicRef") {
        let resolved = ctx.lookup(reference)?;
        if let Value::Object(subschema) = resolved.contents() {
            let validators =
                compile_items_validators(ctx, subschema).map_err(ValidationError::to_owned)?;
            return Ok(Some(Box::new(validators)));
        }
    }
    Ok(None)
}

fn compile_recursive_ref<'a>(
    ctx: &compiler::Context<'_>,
    parent: &Map<String, Value>,
) -> Result<Option<PendingItemsValidators>, ValidationError<'a>> {
    if !parent.contains_key("$recursiveRef") {
        return Ok(None);
    }

    // For $recursiveRef, we need to resolve the reference and check if it's already being compiled
    let resolved = ctx
        .lookup_recursive_reference()
        .map_err(ValidationError::from)?;

    // Create context for the resolved reference and check its cache key
    let (contents, resolver, draft) = resolved.into_inner();
    if let Value::Object(subschema) = &contents {
        let vocabularies = ctx.registry.find_vocabularies(draft, contents);
        let ref_ctx =
            ctx.with_resolver_and_draft(resolver, draft, vocabularies, ctx.location().clone());

        // Check if we're already compiling this schema (circular reference)
        if let Some(pending) = ref_ctx.get_pending_items_validators_for_schema(subschema) {
            return Ok(Some(pending));
        }

        let cache_key = ref_ctx.location_cache_key();
        if let Some(pending) = ref_ctx.get_pending_items_validators(&cache_key) {
            // Circular reference detected - return the pending node
            return Ok(Some(pending));
        }

        // Not circular, compile normally
        let validators =
            compile_items_validators(&ref_ctx, subschema).map_err(ValidationError::to_owned)?;
        let pending = Arc::new(OnceLock::new());
        let _ = pending.set(validators);
        Ok(Some(pending))
    } else {
        Ok(None)
    }
}

fn compile_items<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<(Option<usize>, bool), ValidationError<'a>> {
    if let Some(subschema) = parent.get("items") {
        if ctx.draft() == Draft::Draft201909
            || ctx.draft() == Draft::Draft7
            || ctx.draft() == Draft::Draft6
            || ctx.draft() == Draft::Draft4
        {
            // Older drafts: items can be array or object
            let limit = if parent.contains_key("additionalItems") || subschema.is_object() {
                usize::MAX
            } else {
                subschema.as_array().map_or(usize::MAX, std::vec::Vec::len)
            };
            Ok((Some(limit), false))
        } else {
            // Draft 2020-12+: items is always a schema that applies to all items
            Ok((None, true))
        }
    } else {
        Ok((None, false))
    }
}

fn compile_prefix_items<'a>(
    _ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<usize>, ValidationError<'a>> {
    if let Some(Some(items)) = parent.get("prefixItems").map(Value::as_array) {
        Ok(Some(items.len()))
    } else {
        Ok(None)
    }
}

fn compile_conditional<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<Box<ConditionalValidators>>, ValidationError<'a>> {
    if let Some(subschema) = parent.get("if") {
        if let Value::Object(if_parent) = subschema {
            let if_ctx = ctx.new_at_location("if");

            let mut then_ = None;
            if let Some(Value::Object(subschema)) = parent.get("then") {
                let then_ctx = ctx.new_at_location("then");
                then_ = Some(
                    compile_items_validators(&then_ctx, subschema)
                        .map_err(ValidationError::to_owned)?,
                );
            }

            let mut else_ = None;
            if let Some(Value::Object(subschema)) = parent.get("else") {
                let else_ctx = ctx.new_at_location("else");
                else_ = Some(
                    compile_items_validators(&else_ctx, subschema)
                        .map_err(ValidationError::to_owned)?,
                );
            }

            return Ok(Some(Box::new(ConditionalValidators {
                condition: compiler::compile(&if_ctx, if_ctx.as_resource_ref(subschema))
                    .map_err(ValidationError::to_owned)?,
                if_: compile_items_validators(&if_ctx, if_parent)
                    .map_err(ValidationError::to_owned)?,
                then_,
                else_,
            })));
        }
    }
    Ok(None)
}

fn compile_all_of<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<Vec<(SchemaNode, ItemsValidators)>>, ValidationError<'a>> {
    if let Some(Some(subschemas)) = parent.get("allOf").map(Value::as_array) {
        let all_of_ctx = ctx.new_at_location("allOf");
        let mut result = Vec::with_capacity(subschemas.len());

        for (idx, subschema) in subschemas.iter().enumerate() {
            if let Value::Object(parent) = subschema {
                let subschema_ctx = all_of_ctx.new_at_location(idx);
                result.push((
                    compiler::compile(&subschema_ctx, subschema_ctx.as_resource_ref(subschema))
                        .map_err(ValidationError::to_owned)?,
                    compile_items_validators(&subschema_ctx, parent)
                        .map_err(ValidationError::to_owned)?,
                ));
            }
        }

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

fn compile_any_of<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<Vec<(SchemaNode, ItemsValidators)>>, ValidationError<'a>> {
    if let Some(Some(subschemas)) = parent.get("anyOf").map(Value::as_array) {
        let any_of_ctx = ctx.new_at_location("anyOf");
        let mut result = Vec::with_capacity(subschemas.len());

        for (idx, subschema) in subschemas.iter().enumerate() {
            if let Value::Object(parent) = subschema {
                let subschema_ctx = any_of_ctx.new_at_location(idx);
                result.push((
                    compiler::compile(&subschema_ctx, subschema_ctx.as_resource_ref(subschema))
                        .map_err(ValidationError::to_owned)?,
                    compile_items_validators(&subschema_ctx, parent)
                        .map_err(ValidationError::to_owned)?,
                ));
            }
        }

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

fn compile_one_of<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<Vec<(SchemaNode, ItemsValidators)>>, ValidationError<'a>> {
    if let Some(Some(subschemas)) = parent.get("oneOf").map(Value::as_array) {
        let one_of_ctx = ctx.new_at_location("oneOf");
        let mut result = Vec::with_capacity(subschemas.len());

        for (idx, subschema) in subschemas.iter().enumerate() {
            if let Value::Object(parent) = subschema {
                let subschema_ctx = one_of_ctx.new_at_location(idx);
                result.push((
                    compiler::compile(&subschema_ctx, subschema_ctx.as_resource_ref(subschema))
                        .map_err(ValidationError::to_owned)?,
                    compile_items_validators(&subschema_ctx, parent)
                        .map_err(ValidationError::to_owned)?,
                ));
            }
        }

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Validator for the `unevaluatedItems` keyword.
pub(crate) struct UnevaluatedItemsValidator {
    location: Location,
    validators: ItemsValidators,
}

impl UnevaluatedItemsValidator {
    pub(crate) fn compile<'a>(
        ctx: &'a compiler::Context,
        parent: &'a Map<String, Value>,
    ) -> CompilationResult<'a> {
        let validators =
            compile_items_validators(ctx, parent).map_err(ValidationError::to_owned)?;

        Ok(Box::new(UnevaluatedItemsValidator {
            location: ctx.location().join("unevaluatedItems"),
            validators,
        }))
    }
}

impl Validate for UnevaluatedItemsValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            let mut indexes = vec![false; items.len()];
            self.validators
                .mark_evaluated_indexes(instance, &mut indexes, ctx);

            for (item, is_evaluated) in items.iter().zip(indexes) {
                if !is_evaluated {
                    if let Some(validator) = &self.validators.unevaluated {
                        if !validator.is_valid(item, ctx) {
                            return false;
                        }
                    } else {
                        // unevaluatedItems: false and item not evaluated
                        return false;
                    }
                }
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
        if let Value::Array(items) = instance {
            let mut indexes = vec![false; items.len()];
            self.validators
                .mark_evaluated_indexes(instance, &mut indexes, ctx);
            let mut unevaluated = vec![];

            for (item, is_evaluated) in items.iter().zip(indexes) {
                if !is_evaluated {
                    let is_valid = if let Some(validator) = &self.validators.unevaluated {
                        validator.is_valid(item, ctx)
                    } else {
                        false
                    };

                    if !is_valid {
                        unevaluated.push(item.to_string());
                    }
                }
            }

            if !unevaluated.is_empty() {
                return Err(ValidationError::unevaluated_items(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    unevaluated,
                ));
            }
        }
        Ok(())
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Array(items) = instance {
            let mut indexes = vec![false; items.len()];
            self.validators
                .mark_evaluated_indexes(instance, &mut indexes, ctx);
            let mut children = Vec::new();
            let mut unevaluated = Vec::new();
            let mut invalid = false;

            for (idx, (item, is_evaluated)) in items.iter().zip(indexes.iter()).enumerate() {
                if *is_evaluated {
                    continue;
                }
                if let Some(validator) = &self.validators.unevaluated {
                    let child =
                        validator.evaluate_instance(item, &location.push(idx), tracker, ctx);
                    if !child.valid {
                        invalid = true;
                        unevaluated.push(item.to_string());
                    }
                    children.push(child);
                } else {
                    invalid = true;
                    unevaluated.push(item.to_string());
                }
            }

            let mut errors = Vec::new();
            if !unevaluated.is_empty() {
                errors.push(ErrorDescription::from_validation_error(
                    &ValidationError::unevaluated_items(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        location.into(),
                        instance,
                        unevaluated,
                    ),
                ));
            }

            if invalid {
                EvaluationResult::Invalid {
                    errors,
                    children,
                    annotations: None,
                }
            } else {
                EvaluationResult::Valid {
                    annotations: None,
                    children,
                }
            }
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

pub(crate) fn compile<'a>(
    ctx: &'a compiler::Context,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a>> {
    match schema.as_bool() {
        Some(true) => None,
        _ => Some(UnevaluatedItemsValidator::compile(ctx, parent)),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_unevaluated_items_with_recursion() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "allOf": [
                {
                    "$ref": "#/$defs/array_1"
                }
            ],
            "unevaluatedItems": false,
            "$defs": {
                "array_1": {
                    "type": "array",
                    "prefixItems": [
                        {
                            "type": "string"
                        },
                        {
                            "allOf": [
                                {
                                    "$ref": "#/$defs/array_2"
                                }
                            ],
                            "type": "array",
                            "unevaluatedItems": false
                        }
                    ]
                },
                "array_2": {
                    "type": "array",
                    "prefixItems": [
                        {
                            "type": "number"
                        },
                        {
                            "allOf": [
                                {
                                    "$ref": "#/$defs/array_1"
                                }
                            ],
                            "type": "array",
                            "unevaluatedItems": false
                        }
                    ]
                }
            }
        });

        let validator = crate::validator_for(&schema).expect("Schema should compile");

        // This instance should fail validation because the nested array has an unevaluated item
        let instance = json!([
            "string",
            [
                42,
                [
                    "string",
                    [
                        42,
                        "unexpected" // This item should cause validation to fail
                    ]
                ]
            ]
        ]);

        assert!(!validator.is_valid(&instance));
        assert!(validator.validate(&instance).is_err());

        // This instance should pass validation as all items are evaluated
        let valid_instance = json!(["string", [42, ["string", [42]]]]);

        assert!(validator.is_valid(&valid_instance));
        assert!(validator.validate(&valid_instance).is_ok());
    }
}
