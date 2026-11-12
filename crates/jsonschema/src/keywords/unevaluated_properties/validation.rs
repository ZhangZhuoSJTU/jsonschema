use super::*;

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
