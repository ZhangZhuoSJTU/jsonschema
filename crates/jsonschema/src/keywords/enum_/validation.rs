use super::*;

impl Validate for EnumValidator {
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
            Err(enumeration_error(
                &self.location,
                &self.options,
                instance,
                location,
                tracker,
            ))
        }
    }

    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        // If the input value type is not in the types present among the enum options, then there
        // is no reason to compare it against all items - we know that
        // there are no items with such type at all
        if self.types.contains_value_type(instance) {
            self.items.iter().any(|item| cmp::equal(instance, item))
        } else {
            false
        }
    }
}

impl Validate for SingleValueEnumValidator {
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
            Err(enumeration_error(
                &self.location,
                &self.options,
                instance,
                location,
                tracker,
            ))
        }
    }

    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        cmp::equal(&self.value, instance)
    }
}
