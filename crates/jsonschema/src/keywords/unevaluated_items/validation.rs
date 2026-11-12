use super::*;

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
