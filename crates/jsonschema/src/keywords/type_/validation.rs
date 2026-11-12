use super::*;

impl Validate for MultipleTypesValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        self.types.contains_value_type(instance)
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
            Err(ValidationError::multiple_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                self.types,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if self.is_valid(instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            let message = format!("{instance} is not of types {:?}", self.types);
            EvaluationResult::invalid_empty(vec![ErrorDescription::new("type", message)])
        }
    }
}

impl Validate for NullTypeValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        instance.is_null()
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                JsonType::Null,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if self.is_valid(instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{instance} is not of type "null""#),
            )])
        }
    }
}

impl Validate for BooleanTypeValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        instance.is_boolean()
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                JsonType::Boolean,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if self.is_valid(instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{instance} is not of type "boolean""#),
            )])
        }
    }
}

impl Validate for StringTypeValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        instance.is_string()
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                JsonType::String,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if self.is_valid(instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{instance} is not of type "string""#),
            )])
        }
    }
}

impl Validate for ArrayTypeValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        instance.is_array()
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                JsonType::Array,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if self.is_valid(instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{instance} is not of type "array""#),
            )])
        }
    }
}

impl Validate for ObjectTypeValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        instance.is_object()
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                JsonType::Object,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if self.is_valid(instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{instance} is not of type "object""#),
            )])
        }
    }
}

impl Validate for NumberTypeValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        instance.is_number()
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
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                JsonType::Number,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if self.is_valid(instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{instance} is not of type "number""#),
            )])
        }
    }
}

impl Validate for IntegerTypeValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Number(num) = instance {
            is_integer(num)
        } else {
            false
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
        } else {
            Err(ValidationError::single_type_error(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                JsonType::Integer,
            ))
        }
    }
    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if self.is_valid(instance, ctx) {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(vec![ErrorDescription::new(
                "type",
                format!(r#"{instance} is not of type "integer""#),
            )])
        }
    }
}
