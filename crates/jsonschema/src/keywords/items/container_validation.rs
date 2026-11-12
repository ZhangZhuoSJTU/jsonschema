use super::*;

impl Validate for ItemsArrayValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            for (item, node) in items.iter().zip(self.items.iter()) {
                if !node.is_valid(item, ctx) {
                    return false;
                }
            }
            true
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
            for (idx, (item, node)) in items.iter().zip(self.items.iter()).enumerate() {
                node.validate(item, &location.push(idx), tracker, ctx)?;
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
        if let Value::Array(items) = instance {
            let mut errors = Vec::new();
            for (idx, (item, node)) in items.iter().zip(self.items.iter()).enumerate() {
                errors.extend(node.iter_errors(item, &location.push(idx), tracker, ctx));
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
        if let Value::Array(items) = instance {
            let mut children = Vec::with_capacity(self.items.len().min(items.len()));
            for (idx, (item, node)) in items.iter().zip(self.items.iter()).enumerate() {
                children.push(node.evaluate_instance(item, &location.push(idx), tracker, ctx));
            }
            EvaluationResult::from_children(children)
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

impl Validate for ItemsObjectValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            items.iter().all(|i| self.node.is_valid(i, ctx))
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
            for (idx, item) in items.iter().enumerate() {
                self.node
                    .validate(item, &location.push(idx), tracker, ctx)?;
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
        if let Value::Array(items) = instance {
            let mut errors = Vec::new();
            for (idx, item) in items.iter().enumerate() {
                errors.extend(
                    self.node
                        .iter_errors(item, &location.push(idx), tracker, ctx),
                );
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
        if let Value::Array(items) = instance {
            let mut children = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                children.push(
                    self.node
                        .evaluate_instance(item, &location.push(idx), tracker, ctx),
                );
            }
            let schema_was_applied = !items.is_empty();
            let mut result = EvaluationResult::from_children(children);
            result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
            result
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

impl Validate for ItemsObjectSkipPrefixValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            items
                .iter()
                .skip(self.skip_prefix)
                .all(|i| self.node.is_valid(i, ctx))
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
            for (idx, item) in items.iter().skip(self.skip_prefix).enumerate() {
                self.node
                    .validate(item, &location.push(idx + self.skip_prefix), tracker, ctx)?;
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
        if let Value::Array(items) = instance {
            let mut errors = Vec::new();
            for (idx, item) in items.iter().skip(self.skip_prefix).enumerate() {
                errors.extend(self.node.iter_errors(
                    item,
                    &location.push(idx + self.skip_prefix),
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
        if let Value::Array(items) = instance {
            let mut children = Vec::with_capacity(items.len().saturating_sub(self.skip_prefix));
            for (idx, item) in items.iter().enumerate().skip(self.skip_prefix) {
                children.push(
                    self.node
                        .evaluate_instance(item, &location.push(idx), tracker, ctx),
                );
            }
            let schema_was_applied = items.len() > self.skip_prefix;
            let mut result = EvaluationResult::from_children(children);
            result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
            result
        } else {
            EvaluationResult::valid_empty()
        }
    }
}
