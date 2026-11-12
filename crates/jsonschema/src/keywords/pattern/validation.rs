use super::*;

impl Validate for PrefixPatternValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::String(item) = instance {
            item.starts_with(self.prefix.as_ref())
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
        if let Value::String(item) = instance {
            if !item.starts_with(self.prefix.as_ref()) {
                return Err(ValidationError::pattern(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    self.pattern.to_string(),
                ));
            }
        }
        Ok(())
    }
}
