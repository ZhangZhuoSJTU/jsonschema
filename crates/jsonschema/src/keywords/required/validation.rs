use super::*;

impl Validate for RequiredValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Object(item) = instance {
            if item.len() < self.required.len() {
                return false;
            }
            self.required
                .iter()
                .all(|property_name| item.contains_key(property_name))
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
        if let Value::Object(item) = instance {
            for property_name in &self.required {
                if !item.contains_key(property_name) {
                    return Err(ValidationError::required(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        location.into(),
                        instance,
                        Value::String(property_name.clone()),
                    ));
                }
            }
        }
        Ok(())
    }
    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        if let Value::Object(item) = instance {
            let mut errors = vec![];
            let eval_path = crate::paths::capture_evaluation_path(tracker, &self.location);
            for property_name in &self.required {
                if !item.contains_key(property_name) {
                    errors.push(ValidationError::required(
                        self.location.clone(),
                        eval_path.clone(),
                        location.into(),
                        instance,
                        Value::String(property_name.clone()),
                    ));
                }
            }
            if !errors.is_empty() {
                return ErrorIterator::from_iterator(errors.into_iter());
            }
        }
        no_error()
    }
}

impl Validate for SingleItemRequiredValidator {
    fn validate<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if !self.is_valid(instance, ctx) {
            return Err(ValidationError::required(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                Value::String(self.value.clone()),
            ));
        }
        Ok(())
    }

    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Object(item) = instance {
            if item.is_empty() {
                return false;
            }
            item.contains_key(&self.value)
        } else {
            true
        }
    }
}

impl Validate for Required2Validator {
    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Object(item) = instance {
            item.len() >= 2 && item.contains_key(&self.first) && item.contains_key(&self.second)
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
        if let Value::Object(item) = instance {
            if !item.contains_key(&self.first) {
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    Value::String(self.first.clone()),
                ));
            }
            if !item.contains_key(&self.second) {
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    Value::String(self.second.clone()),
                ));
            }
        }
        Ok(())
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        if let Value::Object(item) = instance {
            let eval_path = crate::paths::capture_evaluation_path(tracker, &self.location);
            let mut errors = Vec::new();
            if !item.contains_key(&self.first) {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path.clone(),
                    location.into(),
                    instance,
                    Value::String(self.first.clone()),
                ));
            }
            if !item.contains_key(&self.second) {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path,
                    location.into(),
                    instance,
                    Value::String(self.second.clone()),
                ));
            }
            if !errors.is_empty() {
                return ErrorIterator::from_iterator(errors.into_iter());
            }
        }
        no_error()
    }
}

impl Validate for Required3Validator {
    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Object(item) = instance {
            item.len() >= 3
                && item.contains_key(&self.first)
                && item.contains_key(&self.second)
                && item.contains_key(&self.third)
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
        if let Value::Object(item) = instance {
            if !item.contains_key(&self.first) {
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    Value::String(self.first.clone()),
                ));
            }
            if !item.contains_key(&self.second) {
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    Value::String(self.second.clone()),
                ));
            }
            if !item.contains_key(&self.third) {
                return Err(ValidationError::required(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    Value::String(self.third.clone()),
                ));
            }
        }
        Ok(())
    }

    fn iter_errors<'i>(
        &self,
        instance: &'i Value,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> ErrorIterator<'i> {
        if let Value::Object(item) = instance {
            let eval_path = crate::paths::capture_evaluation_path(tracker, &self.location);
            let mut errors = Vec::new();
            if !item.contains_key(&self.first) {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path.clone(),
                    location.into(),
                    instance,
                    Value::String(self.first.clone()),
                ));
            }
            if !item.contains_key(&self.second) {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path.clone(),
                    location.into(),
                    instance,
                    Value::String(self.second.clone()),
                ));
            }
            if !item.contains_key(&self.third) {
                errors.push(ValidationError::required(
                    self.location.clone(),
                    eval_path,
                    location.into(),
                    instance,
                    Value::String(self.third.clone()),
                ));
            }
            if !errors.is_empty() {
                return ErrorIterator::from_iterator(errors.into_iter());
            }
        }
        no_error()
    }
}
