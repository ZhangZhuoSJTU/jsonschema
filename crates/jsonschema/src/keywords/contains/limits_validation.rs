use super::*;

impl Validate for MaxContainsValidator {
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
                    if matches > self.max_contains {
                        return false;
                    }
                }
            }
            matches != 0
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
            let loc = self.node.location();
            let mut matches = 0;
            for item in items {
                if self
                    .node
                    .validators()
                    .all(|validator| validator.is_valid(item, ctx))
                {
                    matches += 1;
                    if matches > self.max_contains {
                        return Err(ValidationError::contains(
                            loc.clone(),
                            crate::paths::capture_evaluation_path(tracker, loc),
                            location.into(),
                            instance,
                        ));
                    }
                }
            }
            if matches > 0 {
                Ok(())
            } else {
                Err(ValidationError::contains(
                    loc.clone(),
                    crate::paths::capture_evaluation_path(tracker, loc),
                    location.into(),
                    instance,
                ))
            }
        } else {
            Ok(())
        }
    }
}

impl Validate for MinMaxContainsValidator {
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
                    if matches > self.max_contains {
                        return false;
                    }
                }
            }
            matches <= self.max_contains && matches >= self.min_contains
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
                    if matches > self.max_contains {
                        let max_location = self.node.location().join("maxContains");
                        let eval_path =
                            crate::paths::capture_evaluation_path(tracker, &max_location);
                        return Err(ValidationError::contains(
                            max_location,
                            eval_path,
                            location.into(),
                            instance,
                        ));
                    }
                }
            }
            if matches < self.min_contains {
                let min_location = self.node.location().join("minContains");
                let eval_path = crate::paths::capture_evaluation_path(tracker, &min_location);
                Err(ValidationError::contains(
                    min_location,
                    eval_path,
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
