use super::*;

impl Validate for CustomKeyword {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        instance_path: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        self.inner
            .validate(instance)
            .map_err(|err| err.with_context(instance, instance_path, &self.location, &self.keyword))
    }

    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        self.inner.is_valid(instance)
    }
}
