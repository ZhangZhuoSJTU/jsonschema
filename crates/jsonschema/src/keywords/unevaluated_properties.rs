//! Implementation of the `unevaluatedProperties` keyword.
//!
//! This keyword validates properties that were not evaluated by other keywords like
//! `properties`, `additionalProperties`, `patternProperties`, or nested schemas in
//! combinators (`allOf`, `anyOf`, `oneOf`), conditionals, and references.
//!
//! The implementation eagerly compiles a recursive `PropertyValidators` structure during
//! schema compilation, using `Arc<OnceLock>` for circular reference handling.
use ahash::AHashSet;
use fancy_regex::Regex;
use serde_json::{Map, Value};
use std::sync::{Arc, OnceLock};

use crate::{
    compiler, ecma,
    evaluation::ErrorDescription,
    node::SchemaNode,
    paths::{LazyEvaluationPath, LazyLocation, Location, RefTracker},
    validator::{EvaluationResult, Validate, ValidationContext},
    ValidationError,
};

use super::CompilationResult;

/// Lazy property validators that are compiled on first access.
/// Used for $recursiveRef and circular references to handle cycles during compilation.
pub(crate) type PendingPropertyValidators = Arc<OnceLock<PropertyValidators>>;

/// Holds compiled validators for property evaluation in unevaluatedProperties.
/// This structure is built during schema compilation and used during validation.
#[derive(Debug, Clone)]
pub(crate) struct PropertyValidators {
    /// Property names from "properties" keyword for O(1) lookup
    properties: AHashSet<String>,
    /// Validator from "additionalProperties" keyword
    additional: Option<SchemaNode>,
    /// Pattern-based property validators from "patternProperties" keyword
    pattern_properties: Vec<(Regex, SchemaNode)>,
    /// Validator from "unevaluatedProperties" keyword itself
    unevaluated: Option<SchemaNode>,
    /// Validators from "allOf" keyword - both the schema and its property validators
    all_of: Vec<(SchemaNode, PropertyValidators)>,
    /// Validators from "anyOf" keyword
    any_of: Vec<(SchemaNode, PropertyValidators)>,
    /// Validators from "oneOf" keyword
    one_of: Vec<(SchemaNode, PropertyValidators)>,
    /// Conditional validators from "if/then/else" keywords
    conditional: Option<Box<ConditionalValidators>>,
    /// Reference validators from "$ref" keyword (may be circular)
    ref_: Option<RefValidator>,
    /// Reference validators from "$dynamicRef" keyword
    dynamic_ref: Option<Box<PropertyValidators>>,
    /// Validators from "$recursiveRef" keyword (Draft 2019-09 only)
    /// Uses pending pattern to handle circular references
    recursive_ref: Option<PendingPropertyValidators>,
    /// Dependent schema validators from "dependentSchemas" keyword
    dependent: Vec<(String, PropertyValidators)>,
}

/// Reference validator - just wraps `PropertyValidators`
/// Circular references are handled by returning None during compilation
#[derive(Debug, Clone)]
struct RefValidator(Box<PropertyValidators>);

/// Conditional validators from "if/then/else" keywords
#[derive(Debug, Clone)]
struct ConditionalValidators {
    condition: SchemaNode,
    if_: PropertyValidators,
    then_: Option<PropertyValidators>,
    else_: Option<PropertyValidators>,
}

impl PropertyValidators {
    /// Mark all properties that are evaluated by this schema.
    /// This is the core logic for determining which properties should not be considered "unevaluated".
    fn mark_evaluated_properties<'i>(
        &self,
        instance: &'i Value,
        properties: &mut AHashSet<&'i String>,
        ctx: &mut ValidationContext,
    ) {
        // References first: a `$ref` target can mark properties anywhere in the
        // instance object, including keys the adjacent keywords never see.
        self.mark_referenced(instance, properties, ctx);

        // Adjacent keywords mark the object's own keys directly.
        self.mark_adjacent(instance, properties, ctx);

        // Applicators contribute marks from their nested schemas.
        self.mark_applicators(instance, properties, ctx);
    }

    /// Recurse into `$ref`, `$recursiveRef` and `$dynamicRef` targets so that
    /// whatever they mark is propagated back to this schema's view.
    fn mark_referenced<'i>(
        &self,
        instance: &'i Value,
        properties: &mut AHashSet<&'i String>,
        ctx: &mut ValidationContext,
    ) {
        if let Some(ref_) = &self.ref_ {
            ref_.0.mark_evaluated_properties(instance, properties, ctx);
        }

        // `$recursiveRef` may still be uninitialized while a circular reference
        // is being compiled; skip it and let the parent track those properties.
        if let Some(recursive_ref) = &self.recursive_ref {
            if let Some(validators) = recursive_ref.get() {
                validators.mark_evaluated_properties(instance, properties, ctx);
            }
        }

        if let Some(dynamic_ref) = &self.dynamic_ref {
            dynamic_ref.mark_evaluated_properties(instance, properties, ctx);
        }
    }

    /// Mark the object keys covered by `properties`, `patternProperties`,
    /// `additionalProperties`, `unevaluatedProperties` and `dependentSchemas`.
    fn mark_adjacent<'i>(
        &self,
        instance: &'i Value,
        properties: &mut AHashSet<&'i String>,
        ctx: &mut ValidationContext,
    ) {
        let Some(obj) = instance.as_object() else {
            return;
        };

        // `properties` names are a straight O(1) lookup over the instance keys.
        for property in obj.keys() {
            if self.properties.contains(property) {
                properties.insert(property);
            }
        }

        // `patternProperties` matches by regex, checked only for unmarked keys.
        if !self.pattern_properties.is_empty() {
            for property in obj.keys() {
                if properties.contains(property) {
                    continue; // Already marked by "properties"
                }
                for (pattern, _) in &self.pattern_properties {
                    if pattern.is_match(property).unwrap_or(false) {
                        properties.insert(property);
                        break;
                    }
                }
            }
        }

        // `additionalProperties` swallows every key the previous two missed.
        // This must run after marking properties/patternProperties to avoid an
        // order dependency on how the instance map is iterated.
        if self.additional.is_some() {
            for property in obj.keys() {
                if !properties.contains(property) {
                    properties.insert(property);
                }
            }
        }

        // `unevaluatedProperties` marks keys that validate against its schema.
        // This matters for nested unevaluatedProperties: a child schema's
        // unevaluatedProperties can mark keys for its parent schemas too.
        if let Some(unevaluated) = &self.unevaluated {
            for (property, value) in obj {
                if properties.contains(property) {
                    continue;
                }
                if unevaluated.is_valid(value, ctx) {
                    properties.insert(property);
                }
            }
        }

        // `dependentSchemas` marks keys of dependent schemas that apply.
        for (dep_property, dep_validators) in &self.dependent {
            if obj.contains_key(dep_property) {
                dep_validators.mark_evaluated_properties(instance, properties, ctx);
            }
        }
    }

    /// Recurse into conditionals and combinators, letting successful branches
    /// contribute their own marks.
    fn mark_applicators<'i>(
        &self,
        instance: &'i Value,
        properties: &mut AHashSet<&'i String>,
        ctx: &mut ValidationContext,
    ) {
        if let Some(conditional) = &self.conditional {
            conditional.mark_evaluated_properties(instance, properties, ctx);
        }

        for (node, validators) in &self.all_of {
            if node.is_valid(instance, ctx) {
                validators.mark_evaluated_properties(instance, properties, ctx);
            }
        }

        for (node, validators) in &self.any_of {
            if node.is_valid(instance, ctx) {
                validators.mark_evaluated_properties(instance, properties, ctx);
            }
        }

        let mut matched = None;
        let mut matches = 0usize;
        for (node, validators) in &self.one_of {
            //just a buggy refactor, hard to catch.
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
            validators.mark_evaluated_properties(instance, properties, ctx);
        }
    }
}

impl ConditionalValidators {
    fn mark_evaluated_properties<'i>(
        &self,
        instance: &'i Value,
        properties: &mut AHashSet<&'i String>,
        ctx: &mut ValidationContext,
    ) {
        if self.condition.is_valid(instance, ctx) {
            self.if_
                .mark_evaluated_properties(instance, properties, ctx);
            if let Some(then_) = &self.then_ {
                then_.mark_evaluated_properties(instance, properties, ctx);
            }
        } else if let Some(else_) = &self.else_ {
            else_.mark_evaluated_properties(instance, properties, ctx);
        }
    }
}

/// Compile all property validators for a schema.
///
/// Recursively builds the `PropertyValidators` tree by examining all keywords that
/// can evaluate properties. Handles circular references via pending nodes cached
/// by location and schema pointer.
fn compile_property_validators<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<PropertyValidators, ValidationError<'a>> {
    // Create a pending node and cache it before compiling to handle circular refs
    let cache_key = ctx.location_cache_key();
    let pending = Arc::new(OnceLock::new());
    ctx.cache_pending_property_validators(cache_key.clone(), pending.clone());
    ctx.cache_pending_property_validators_for_schema(parent, pending.clone());

    // Compile all parts
    let validators = PropertyValidators {
        properties: compile_properties(ctx, parent)?,
        additional: compile_additional(ctx, parent)?,
        pattern_properties: compile_pattern_properties(ctx, parent)?,
        unevaluated: compile_unevaluated(ctx, parent)?,
        all_of: compile_all_of(ctx, parent)?,
        any_of: compile_any_of(ctx, parent)?,
        one_of: compile_one_of(ctx, parent)?,
        conditional: compile_conditional(ctx, parent)?,
        ref_: compile_ref(ctx, parent).map_err(ValidationError::to_owned)?,
        dynamic_ref: compile_dynamic_ref(ctx, parent).map_err(ValidationError::to_owned)?,
        recursive_ref: compile_recursive_ref(ctx, parent)?,
        dependent: compile_dependent(ctx, parent)?,
    };

    // Initialize the pending node. This should always succeed since we just created it.
    pending
        .set(validators.clone())
        .expect("pending node should not be initialized yet");

    // Remove from pending cache
    ctx.remove_pending_property_validators(&cache_key);
    ctx.remove_pending_property_validators_for_schema(parent);

    Ok(validators)
}

fn compile_properties<'a>(
    _ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<AHashSet<String>, ValidationError<'a>> {
    let Some(Value::Object(map)) = parent.get("properties") else {
        return Ok(AHashSet::new());
    };
    // Only need property names for evaluation tracking, not the validators
    Ok(map.keys().cloned().collect())
}

fn compile_additional<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<SchemaNode>, ValidationError<'a>> {
    let Some(subschema) = parent.get("additionalProperties") else {
        return Ok(None);
    };

    let additional_ctx = ctx.new_at_location("additionalProperties");
    let node = compiler::compile(&additional_ctx, additional_ctx.as_resource_ref(subschema))
        .map_err(ValidationError::to_owned)?;
    Ok(Some(node))
}

fn compile_pattern_properties<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Vec<(Regex, SchemaNode)>, ValidationError<'a>> {
    let Some(Value::Object(patterns)) = parent.get("patternProperties") else {
        return Ok(Vec::new());
    };

    let pat_ctx = ctx.new_at_location("patternProperties");
    let mut result = Vec::with_capacity(patterns.len());

    for (pattern, schema) in patterns {
        let schema_ctx = pat_ctx.new_at_location(pattern.as_str());
        let Ok(regex) = ecma::to_rust_regex(pattern).and_then(|p| Regex::new(&p).map_err(|_| ()))
        else {
            return Err(ValidationError::format(
                schema_ctx.location().clone(),
                LazyEvaluationPath::SameAsSchemaPath,
                Location::new(),
                schema,
                "regex",
            ));
        };
        let node = compiler::compile(&schema_ctx, schema_ctx.as_resource_ref(schema))
            .map_err(ValidationError::to_owned)?;
        result.push((regex, node));
    }

    Ok(result)
}

fn compile_unevaluated<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<SchemaNode>, ValidationError<'a>> {
    let Some(subschema) = parent.get("unevaluatedProperties") else {
        return Ok(None);
    };

    let unevaluated_ctx = ctx.new_at_location("unevaluatedProperties");
    let node = compiler::compile(&unevaluated_ctx, unevaluated_ctx.as_resource_ref(subschema))
        .map_err(ValidationError::to_owned)?;
    Ok(Some(node))
}

fn compile_all_of<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Vec<(SchemaNode, PropertyValidators)>, ValidationError<'a>> {
    let Some(Some(subschemas)) = parent.get("allOf").map(Value::as_array) else {
        return Ok(Vec::new());
    };

    let all_of_ctx = ctx.new_at_location("allOf");
    let mut result = Vec::with_capacity(subschemas.len());

    for (idx, subschema) in subschemas.iter().enumerate() {
        let subschema_ctx = all_of_ctx.new_at_location(idx);
        let node = compiler::compile(&subschema_ctx, subschema_ctx.as_resource_ref(subschema))
            .map_err(ValidationError::to_owned)?;

        if let Value::Object(obj) = subschema {
            let validators = compile_property_validators(&subschema_ctx, obj)?;
            result.push((node, validators));
        }
    }

    Ok(result)
}

fn compile_any_of<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Vec<(SchemaNode, PropertyValidators)>, ValidationError<'a>> {
    let Some(Some(subschemas)) = parent.get("anyOf").map(Value::as_array) else {
        return Ok(Vec::new());
    };

    let any_of_ctx = ctx.new_at_location("anyOf");
    let mut result = Vec::with_capacity(subschemas.len());

    for (idx, subschema) in subschemas.iter().enumerate() {
        let subschema_ctx = any_of_ctx.new_at_location(idx);
        let node = compiler::compile(&subschema_ctx, subschema_ctx.as_resource_ref(subschema))
            .map_err(ValidationError::to_owned)?;

        if let Value::Object(obj) = subschema {
            let validators = compile_property_validators(&subschema_ctx, obj)?;
            result.push((node, validators));
        }
    }

    Ok(result)
}

fn compile_one_of<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Vec<(SchemaNode, PropertyValidators)>, ValidationError<'a>> {
    let Some(Some(subschemas)) = parent.get("oneOf").map(Value::as_array) else {
        return Ok(Vec::new());
    };

    let one_of_ctx = ctx.new_at_location("oneOf");
    let mut result = Vec::with_capacity(subschemas.len());

    for (idx, subschema) in subschemas.iter().enumerate() {
        let subschema_ctx = one_of_ctx.new_at_location(idx);
        let node = compiler::compile(&subschema_ctx, subschema_ctx.as_resource_ref(subschema))
            .map_err(ValidationError::to_owned)?;

        if let Value::Object(obj) = subschema {
            let validators = compile_property_validators(&subschema_ctx, obj)?;
            result.push((node, validators));
        }
    }

    Ok(result)
}

fn compile_conditional<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Option<Box<ConditionalValidators>>, ValidationError<'a>> {
    let Some(Value::Object(if_schema)) = parent.get("if") else {
        return Ok(None);
    };

    let if_ctx = ctx.new_at_location("if");
    let condition = compiler::compile(
        &if_ctx,
        if_ctx.as_resource_ref(&Value::Object(if_schema.clone())),
    )
    .map_err(ValidationError::to_owned)?;
    let if_ = compile_property_validators(&if_ctx, if_schema)?;

    let then_ = if let Some(Value::Object(then_schema)) = parent.get("then") {
        let then_ctx = ctx.new_at_location("then");
        Some(compile_property_validators(&then_ctx, then_schema)?)
    } else {
        None
    };

    let else_ = if let Some(Value::Object(else_schema)) = parent.get("else") {
        let else_ctx = ctx.new_at_location("else");
        Some(compile_property_validators(&else_ctx, else_schema)?)
    } else {
        None
    };

    Ok(Some(Box::new(ConditionalValidators {
        condition,
        if_,
        then_,
        else_,
    })))
}

fn compile_ref<'a>(
    ctx: &compiler::Context<'_>,
    parent: &Map<String, Value>,
) -> Result<Option<RefValidator>, ValidationError<'a>> {
    let Some(Value::String(reference)) = parent.get("$ref") else {
        return Ok(None);
    };

    let resolved = ctx.lookup(reference).map_err(ValidationError::from)?;

    let (contents, resolver, draft) = resolved.into_inner();
    if let Value::Object(subschema) = &contents {
        let vocabularies = ctx.registry.find_vocabularies(draft, contents);
        let ref_ctx =
            ctx.with_resolver_and_draft(resolver, draft, vocabularies, ctx.location().clone());
        let validators =
            compile_property_validators(&ref_ctx, subschema).map_err(ValidationError::to_owned)?;
        Ok(Some(RefValidator(Box::new(validators))))
    } else {
        Ok(None)
    }
}

fn compile_dynamic_ref<'a>(
    ctx: &compiler::Context<'_>,
    parent: &Map<String, Value>,
) -> Result<Option<Box<PropertyValidators>>, ValidationError<'a>> {
    let Some(Value::String(reference)) = parent.get("$dynamicRef") else {
        return Ok(None);
    };

    let resolved = ctx.lookup(reference).map_err(ValidationError::from)?;

    let (contents, resolver, draft) = resolved.into_inner();
    if let Value::Object(subschema) = &contents {
        let vocabularies = ctx.registry.find_vocabularies(draft, contents);
        let ref_ctx =
            ctx.with_resolver_and_draft(resolver, draft, vocabularies, ctx.location().clone());
        let validators =
            compile_property_validators(&ref_ctx, subschema).map_err(ValidationError::to_owned)?;
        Ok(Some(Box::new(validators)))
    } else {
        Ok(None)
    }
}

fn compile_recursive_ref<'a>(
    ctx: &compiler::Context<'_>,
    parent: &Map<String, Value>,
) -> Result<Option<PendingPropertyValidators>, ValidationError<'a>> {
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
        if let Some(pending) = ref_ctx.get_pending_property_validators_for_schema(subschema) {
            return Ok(Some(pending));
        }

        let cache_key = ref_ctx.location_cache_key();
        if let Some(pending) = ref_ctx.get_pending_property_validators(&cache_key) {
            // Circular reference detected - return the pending node
            return Ok(Some(pending));
        }

        // Not circular, compile normally
        let validators =
            compile_property_validators(&ref_ctx, subschema).map_err(ValidationError::to_owned)?;
        let pending = Arc::new(OnceLock::new());
        let _ = pending.set(validators);
        Ok(Some(pending))
    } else {
        Ok(None)
    }
}

fn compile_dependent<'a>(
    ctx: &compiler::Context<'_>,
    parent: &'a Map<String, Value>,
) -> Result<Vec<(String, PropertyValidators)>, ValidationError<'a>> {
    let Some(Value::Object(map)) = parent.get("dependentSchemas") else {
        return Ok(Vec::new());
    };

    let dependent_ctx = ctx.new_at_location("dependentSchemas");
    let mut result = Vec::with_capacity(map.len());

    for (property, subschema) in map {
        if let Value::Object(obj) = subschema {
            let property_ctx = dependent_ctx.new_at_location(property.as_str());
            let validators = compile_property_validators(&property_ctx, obj)?;
            result.push((property.clone(), validators));
        }
    }

    Ok(result)
}

/// Validator for the `unevaluatedProperties` keyword.
pub(crate) struct UnevaluatedPropertiesValidator {
    location: Location,
    validators: PropertyValidators,
}

impl UnevaluatedPropertiesValidator {
    pub(crate) fn compile<'a>(
        ctx: &'a compiler::Context,
        parent: &'a Map<String, Value>,
    ) -> CompilationResult<'a> {
        let validators =
            compile_property_validators(ctx, parent).map_err(ValidationError::to_owned)?;

        Ok(Box::new(UnevaluatedPropertiesValidator {
            location: ctx.location().join("unevaluatedProperties"),
            validators,
        }))
    }
}

impl Validate for UnevaluatedPropertiesValidator {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Value::Object(properties) = instance {
            let mut evaluated = AHashSet::with_capacity(properties.len());

            // Mark all evaluated properties
            self.validators
                .mark_evaluated_properties(instance, &mut evaluated, ctx);

            // Early return if all properties are evaluated
            if evaluated.len() == properties.len() {
                return Ok(());
            }

            // Check for unevaluated properties
            let mut unevaluated = Vec::new();
            for (property, value) in properties {
                if evaluated.contains(property) {
                    continue;
                }
                // Check against unevaluatedProperties schema
                if let Some(unevaluated_schema) = &self.validators.unevaluated {
                    if !unevaluated_schema.is_valid(value, ctx) {
                        unevaluated.push(property.clone());
                    }
                } else {
                    // No unevaluatedProperties schema means false (reject all)
                    unevaluated.push(property.clone());
                }
            }

            if !unevaluated.is_empty() {
                return Err(ValidationError::unevaluated_properties(
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

    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Object(properties) = instance {
            let mut evaluated = AHashSet::with_capacity(properties.len());
            self.validators
                .mark_evaluated_properties(instance, &mut evaluated, ctx);

            // Early return if all properties are evaluated
            if evaluated.len() == properties.len() {
                return true;
            }

            for (property, value) in properties {
                if evaluated.contains(property) {
                    continue;
                }
                if let Some(unevaluated_schema) = &self.validators.unevaluated {
                    if !unevaluated_schema.is_valid(value, ctx) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Object(properties) = instance {
            let mut evaluated = AHashSet::with_capacity(properties.len());
            self.validators
                .mark_evaluated_properties(instance, &mut evaluated, ctx);
            let mut children = Vec::new();
            let mut unevaluated = Vec::new();
            let mut invalid = false;

            for (property, value) in properties {
                if evaluated.contains(property) {
                    continue;
                }
                if let Some(validator) = &self.validators.unevaluated {
                    let child =
                        validator.evaluate_instance(value, &location.push(property), tracker, ctx);
                    if !child.valid {
                        invalid = true;
                        unevaluated.push(property.clone());
                    }
                    children.push(child);
                } else {
                    invalid = true;
                    unevaluated.push(property.clone());
                }
            }

            let mut errors = Vec::new();
            if !unevaluated.is_empty() {
                errors.push(ErrorDescription::from_validation_error(
                    &ValidationError::unevaluated_properties(
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
        Some(true) => None, // unevaluatedProperties: true is a no-op
        _ => Some(UnevaluatedPropertiesValidator::compile(ctx, parent)),
    }
}

#[cfg(test)]
mod tests {
    use crate::error::ValidationErrorKind;
    use serde_json::json;

    #[test]
    fn recursive_ref_preserves_unevaluated_properties() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "$id": "https://example.com/root",
            "$recursiveAnchor": true,
            "type": "object",
            "properties": {
                "child": {
                    "type": "object",
                    "properties": {
                        "child": { "$recursiveRef": "#" }
                    },
                    "unevaluatedProperties": false
                }
            },
            "unevaluatedProperties": false
        });

        let validator = crate::options().build(&schema).expect("schema compiles");

        let valid = json!({"child": {"child": {}}});
        assert!(
            validator.is_valid(&valid),
            "expected recursive schema without extras to be valid"
        );

        let invalid = json!({"child": {"child": {"unexpected": 1}}});
        assert!(
            !validator.is_valid(&invalid),
            "unexpected properties should be rejected"
        );

        let errors: Vec<_> = validator.iter_errors(&invalid).collect();
        assert!(
            errors.iter().any(|err| matches!(
                err.kind(),
                ValidationErrorKind::UnevaluatedProperties { .. }
            )),
            "expected unevaluatedProperties error, got {errors:?}"
        );
    }
}
