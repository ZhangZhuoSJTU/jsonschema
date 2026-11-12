use super::*;

impl Validate for FalseValidator {
    fn is_valid(&self, _: &Value, _ctx: &mut ValidationContext) -> bool {
        false
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        Err(ValidationError::false_schema(
            self.location.clone(),
            crate::paths::capture_evaluation_path(tracker, &self.location),
            location.into(),
            instance,
        ))
    }
}
