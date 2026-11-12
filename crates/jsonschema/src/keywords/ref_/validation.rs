use super::*;

impl Validate for RefValidator {
    fn is_valid(&self, instance: &Value, ctx: &mut ValidationContext) -> bool {
        self.inner.is_valid(instance, ctx)
    }

    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner
            .validate(instance, location, Some(&child_tracker), ctx)
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner
            .iter_errors(instance, location, Some(&child_tracker), ctx)
    }

    fn evaluate(
        &self,
        instance: &Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner
            .evaluate(instance, location, Some(&child_tracker), ctx)
    }

    /// Returns `ref_target_base` for `schema_path` output.
    ///
    /// Per JSON Schema 2020-12 Core Section 12.4.2, `schema_path` "MUST NOT include
    /// by-reference applicators such as `$ref` or `$dynamicRef`".
    fn canonical_location(&self) -> Option<&Location> {
        Some(&self.ref_target_base)
    }
}
