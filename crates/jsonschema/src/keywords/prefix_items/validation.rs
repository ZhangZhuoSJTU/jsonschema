use super::*;

impl Validate for PrefixItemsValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            for (schema, item) in self.schemas.iter().zip(items.iter()) {
                if !schema.is_valid(item, ctx) {
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
            for (idx, (schema, item)) in self.schemas.iter().zip(items.iter()).enumerate() {
                schema.validate(item, &location.push(idx), tracker, ctx)?;
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
            for (idx, (schema, item)) in self.schemas.iter().zip(items.iter()).enumerate() {
                errors.extend(schema.iter_errors(item, &location.push(idx), tracker, ctx));
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
            if !items.is_empty() {
                let mut children = Vec::with_capacity(self.schemas.len().min(items.len()));
                let mut max_index_applied = 0usize;
                for (idx, (schema_node, item)) in self.schemas.iter().zip(items.iter()).enumerate()
                {
                    children.push(schema_node.evaluate_instance(
                        item,
                        &location.push(idx),
                        tracker,
                        ctx,
                    ));
                    max_index_applied = idx;
                }
                let annotation = if children.len() == items.len() {
                    Value::Bool(true)
                } else {
                    Value::from(max_index_applied)
                };
                let mut result = EvaluationResult::from_children(children);
                result.annotate(Annotations::new(annotation));
                return result;
            }
        }
        EvaluationResult::valid_empty()
    }
}
