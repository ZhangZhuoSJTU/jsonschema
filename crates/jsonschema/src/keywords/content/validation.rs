use super::*;

/// Validator delegates validation to the stored function.
impl Validate for ContentMediaTypeValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::String(item) = instance {
            (self.func)(item)
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
        if self.is_valid(instance, ctx) {
            Ok(())
        } else if let Value::String(_) = instance {
            let loc = &self.location;
            Err(ValidationError::content_media_type(
                loc.clone(),
                crate::paths::capture_evaluation_path(tracker, loc),
                location.into(),
                instance,
                &self.media_type,
            ))
        } else {
            Ok(())
        }
    }
}

impl Validate for ContentEncodingValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::String(item) = instance {
            (self.func)(item)
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
        if self.is_valid(instance, ctx) {
            Ok(())
        } else if let Value::String(_) = instance {
            let loc = &self.location;
            Err(ValidationError::content_encoding(
                loc.clone(),
                crate::paths::capture_evaluation_path(tracker, loc),
                location.into(),
                instance,
                &self.encoding,
            ))
        } else {
            Ok(())
        }
    }
}

/// Decode the input value & check media type
impl Validate for ContentMediaTypeAndEncodingValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::String(item) = instance {
            match (self.converter)(item) {
                Ok(None) | Err(_) => false,
                Ok(Some(converted)) => (self.func)(&converted),
            }
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
            match (self.converter)(item) {
                Ok(None) => {
                    let encoding_location = self.location.join("contentEncoding");
                    let eval_path =
                        crate::paths::capture_evaluation_path(tracker, &encoding_location);
                    Err(ValidationError::content_encoding(
                        encoding_location,
                        eval_path,
                        location.into(),
                        instance,
                        &self.encoding,
                    ))
                }
                Ok(Some(converted)) => {
                    if (self.func)(&converted) {
                        Ok(())
                    } else {
                        let media_type_location = self.location.join("contentMediaType");
                        let eval_path =
                            crate::paths::capture_evaluation_path(tracker, &media_type_location);
                        Err(ValidationError::content_media_type(
                            media_type_location,
                            eval_path,
                            location.into(),
                            instance,
                            &self.media_type,
                        ))
                    }
                }
                Err(e) => Err(e),
            }
        } else {
            Ok(())
        }
    }
}
