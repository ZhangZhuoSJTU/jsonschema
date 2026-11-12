use super::*;

impl Validate for UniqueItemsValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            if !is_unique(items) {
                return false;
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
        if self.is_valid(instance, ctx) {
            Ok(())
        } else {
            Err(ValidationError::unique_items(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
            ))
        }
    }
}
