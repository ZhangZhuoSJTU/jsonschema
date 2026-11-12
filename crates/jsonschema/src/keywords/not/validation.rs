use super::*;

impl Validate for NotValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        !self.node.is_valid(instance, ctx)
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
            Err(ValidationError::not(
                self.node.location().clone(),
                crate::paths::capture_evaluation_path(tracker, self.node.location()),
                location.into(),
                instance,
                self.original.clone(),
            ))
        }
    }
}
