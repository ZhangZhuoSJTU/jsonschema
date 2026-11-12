use super::*;

impl Validate for ContainsValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            items.iter().any(|i| self.node.is_valid(i, ctx))
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Value::Array(items) = instance {
            if items.iter().any(|i| self.node.is_valid(i, ctx)) {
                return Ok(());
            }
            let loc = self.node.location();
            Err(ValidationError::contains(
                loc.clone(),
                crate::paths::capture_evaluation_path(tracker, loc),
                location.into(),
                instance,
            ))
        } else {
            Ok(())
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Array(items) = instance {
            let mut results = Vec::with_capacity(items.len());
            let mut indices = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                let path = location.push(idx);
                let result = self.node.evaluate_instance(item, &path, tracker, ctx);
                if result.valid {
                    indices.push(idx);
                    results.push(result);
                }
            }
            if indices.is_empty() {
                let loc = self.node.location();
                let eval_path = crate::paths::capture_evaluation_path(tracker, loc);
                EvaluationResult::Invalid {
                    errors: vec![ErrorDescription::from_validation_error(
                        &ValidationError::contains(
                            loc.clone(),
                            eval_path,
                            location.into(),
                            instance,
                        ),
                    )],
                    children: Vec::new(),
                    annotations: None,
                }
            } else {
                EvaluationResult::Valid {
                    annotations: Some(Annotations::new(Value::from(indices))),
                    children: results,
                }
            }
        } else {
            let mut result = EvaluationResult::valid_empty();
            result.annotate(Annotations::new(Value::Array(Vec::new())));
            result
        }
    }
}

impl Validate for MinContainsValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            let mut matches = 0;
            for item in items {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(item, ctx))
                {
                    matches += 1;
                    if matches >= self.min_contains {
                        return true;
                    }
                }
            }
            self.min_contains == 0
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Value::Array(items) = instance {
            let mut matches = 0;
            for item in items {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(item, ctx))
                {
                    matches += 1;
                    if matches >= self.min_contains {
                        return Ok(());
                    }
                }
            }
            if self.min_contains > 0 {
                let loc = self.node.location();
                Err(ValidationError::contains(
                    loc.clone(),
                    crate::paths::capture_evaluation_path(tracker, loc),
                    location.into(),
                    instance,
                ))
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}
