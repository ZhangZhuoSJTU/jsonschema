use super::*;

impl Validate for AdditionalPropertiesValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Object(item) = instance {
            item.values().all(|i| self.node.is_valid(i, ctx))
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
        if let Value::Object(item) = instance {
            for (name, value) in item {
                self.node
                    .validate(value, &location.push(name), tracker, ctx)?;
            }
        }
        Ok(())
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        if let Value::Object(item) = instance {
            let mut errors = Vec::new();
            for (name, value) in item {
                errors.extend(self.node.iter_errors(
                    value,
                    &location.push(name.as_str()),
                    tracker,
                    ctx,
                ));
            }
            ErrorIterator::from_iterator(errors.into_iter())
        } else {
            no_error()
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Object(item) = instance {
            let mut children = Vec::with_capacity(item.len());
            for (name, value) in item {
                children.push(self.node.evaluate_instance(
                    value,
                    &location.push(name.as_str()),
                    tracker,
                    ctx,
                ));
            }
            let mut result = EvaluationResult::from_children(children);
            let annotated_props = item
                .keys()
                .cloned()
                .map(serde_json::Value::String)
                .collect();
            result.annotate(Annotations::new(serde_json::Value::Array(annotated_props)));
            result
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

impl Validate for AdditionalPropertiesFalseValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Object(item) = instance {
            item.iter().next().is_none()
        } else {
            true
        }
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Value::Object(item) = instance {
            if let Some((_, value)) = item.iter().next() {
                return Err(ValidationError::false_schema(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    value,
                ));
            }
        }
        Ok(())
    }
}
