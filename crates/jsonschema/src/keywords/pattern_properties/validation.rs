use super::*;

impl Validate for PrefixPatternPropertiesValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Object(item) = instance {
            for (prefix, node) in &self.prefixes {
                for (key, value) in item {
                    if key.starts_with(prefix.as_ref()) && !node.is_valid(value, ctx) {
                        return false;
                    }
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
        if let Value::Object(item) = instance {
            for (key, value) in item {
                for (prefix, node) in &self.prefixes {
                    if key.starts_with(prefix.as_ref()) {
                        node.validate(value, &location.push(key), tracker, ctx)?;
                    }
                }
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
            for (prefix, node) in &self.prefixes {
                for (key, value) in item {
                    if key.starts_with(prefix.as_ref()) {
                        errors.extend(node.iter_errors(
                            value,
                            &location.push(key.as_str()),
                            tracker,
                            ctx,
                        ));
                    }
                }
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
            let mut matched_propnames = Vec::with_capacity(item.len());
            let mut children = Vec::new();
            for (prefix, node) in &self.prefixes {
                for (key, value) in item {
                    if key.starts_with(prefix.as_ref()) {
                        matched_propnames.push(key.clone());
                        children.push(node.evaluate_instance(
                            value,
                            &location.push(key.as_str()),
                            tracker,
                            ctx,
                        ));
                    }
                }
            }
            let mut result = EvaluationResult::from_children(children);
            result.annotate(Annotations::new(Value::from(matched_propnames)));
            result
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

impl Validate for SinglePrefixPatternPropertiesValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Object(item) = instance {
            for (key, value) in item {
                if key.starts_with(self.prefix.as_ref()) && !self.node.is_valid(value, ctx) {
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
        if let Value::Object(item) = instance {
            for (key, value) in item {
                if key.starts_with(self.prefix.as_ref()) {
                    self.node
                        .validate(value, &location.push(key), tracker, ctx)?;
                }
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
            for (key, value) in item {
                if key.starts_with(self.prefix.as_ref()) {
                    errors.extend(self.node.iter_errors(
                        value,
                        &location.push(key.as_str()),
                        tracker,
                        ctx,
                    ));
                }
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
            let mut matched_propnames = Vec::with_capacity(item.len());
            let mut children = Vec::new();
            for (key, value) in item {
                if key.starts_with(self.prefix.as_ref()) {
                    matched_propnames.push(key.clone());
                    children.push(self.node.evaluate_instance(
                        value,
                        &location.push(key.as_str()),
                        tracker,
                        ctx,
                    ));
                }
            }
            let mut result = EvaluationResult::from_children(children);
            result.annotate(Annotations::new(Value::from(matched_propnames)));
            result
        } else {
            EvaluationResult::valid_empty()
        }
    }
}
