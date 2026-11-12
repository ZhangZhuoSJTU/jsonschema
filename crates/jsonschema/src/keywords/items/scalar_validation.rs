use super::*;

impl Validate for ItemsNumberTypeValidator {
    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            items.iter().all(Value::is_number)
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
        if let Value::Array(items) = instance {
            for (idx, item) in items.iter().enumerate() {
                if !item.is_number() {
                    return Err(ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::Number,
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
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.is_number())
                .map(|(idx, item)| {
                    ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::Number,
                    )
                })
                .collect();
            ErrorIterator::from_iterator(errors.into_iter())
        } else {
            no_error()
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.is_number())
                .map(|(idx, item)| {
                    ErrorDescription::new(
                        "type",
                        format!(r#"{item} at index {idx} is not of type "number""#),
                    )
                })
                .collect();
            let schema_was_applied = !items.is_empty();
            if errors.is_empty() {
                let mut result = EvaluationResult::valid_empty();
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            } else {
                let mut result = EvaluationResult::invalid_empty(errors);
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            }
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

impl Validate for ItemsStringTypeValidator {
    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            items.iter().all(Value::is_string)
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
        if let Value::Array(items) = instance {
            for (idx, item) in items.iter().enumerate() {
                if !item.is_string() {
                    return Err(ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::String,
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
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.is_string())
                .map(|(idx, item)| {
                    ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::String,
                    )
                })
                .collect();
            ErrorIterator::from_iterator(errors.into_iter())
        } else {
            no_error()
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.is_string())
                .map(|(idx, item)| {
                    ErrorDescription::new(
                        "type",
                        format!(r#"{item} at index {idx} is not of type "string""#),
                    )
                })
                .collect();
            let schema_was_applied = !items.is_empty();
            if errors.is_empty() {
                let mut result = EvaluationResult::valid_empty();
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            } else {
                let mut result = EvaluationResult::invalid_empty(errors);
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            }
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

impl Validate for ItemsIntegerTypeValidator {
    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            items.iter().all(|item| {
                if let Value::Number(n) = item {
                    super::super::type_::is_integer(n)
                } else {
                    false
                }
            })
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
        if let Value::Array(items) = instance {
            for (idx, item) in items.iter().enumerate() {
                let valid = if let Value::Number(n) = item {
                    super::super::type_::is_integer(n)
                } else {
                    false
                };
                if !valid {
                    return Err(ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::Integer,
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
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if let Value::Number(n) = item {
                        !super::super::type_::is_integer(n)
                    } else {
                        true
                    }
                })
                .map(|(idx, item)| {
                    ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::Integer,
                    )
                })
                .collect();
            ErrorIterator::from_iterator(errors.into_iter())
        } else {
            no_error()
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if let Value::Number(n) = item {
                        !super::super::type_::is_integer(n)
                    } else {
                        true
                    }
                })
                .map(|(idx, item)| {
                    ErrorDescription::new(
                        "type",
                        format!(r#"{item} at index {idx} is not of type "integer""#),
                    )
                })
                .collect();
            let schema_was_applied = !items.is_empty();
            if errors.is_empty() {
                let mut result = EvaluationResult::valid_empty();
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            } else {
                let mut result = EvaluationResult::invalid_empty(errors);
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            }
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

impl Validate for ItemsIntegerTypeValidatorDraft4 {
    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            items.iter().all(|item| {
                if let Value::Number(n) = item {
                    super::super::legacy::type_draft_4::is_integer(n)
                } else {
                    false
                }
            })
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
        if let Value::Array(items) = instance {
            for (idx, item) in items.iter().enumerate() {
                let valid = if let Value::Number(n) = item {
                    super::super::legacy::type_draft_4::is_integer(n)
                } else {
                    false
                };
                if !valid {
                    return Err(ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::Integer,
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
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if let Value::Number(n) = item {
                        !super::super::legacy::type_draft_4::is_integer(n)
                    } else {
                        true
                    }
                })
                .map(|(idx, item)| {
                    ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::Integer,
                    )
                })
                .collect();
            ErrorIterator::from_iterator(errors.into_iter())
        } else {
            no_error()
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if let Value::Number(n) = item {
                        !super::super::legacy::type_draft_4::is_integer(n)
                    } else {
                        true
                    }
                })
                .map(|(idx, item)| {
                    ErrorDescription::new(
                        "type",
                        format!(r#"{item} at index {idx} is not of type "integer""#),
                    )
                })
                .collect();
            let schema_was_applied = !items.is_empty();
            if errors.is_empty() {
                let mut result = EvaluationResult::valid_empty();
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            } else {
                let mut result = EvaluationResult::invalid_empty(errors);
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            }
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

impl Validate for ItemsBooleanTypeValidator {
    #[inline]
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Array(items) = instance {
            items.iter().all(Value::is_boolean)
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
        if let Value::Array(items) = instance {
            for (idx, item) in items.iter().enumerate() {
                if !item.is_boolean() {
                    return Err(ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::Boolean,
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
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.is_boolean())
                .map(|(idx, item)| {
                    ValidationError::single_type_error(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        (&location.push(idx)).into(),
                        item,
                        JsonType::Boolean,
                    )
                })
                .collect();
            ErrorIterator::from_iterator(errors.into_iter())
        } else {
            no_error()
        }
    }

    fn evaluate(
        &self,
        instance: &Value,
        _location: &LazyLocation,
        _tracker: Option<&RefTracker>,
        _ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Value::Array(items) = instance {
            let errors: Vec<_> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.is_boolean())
                .map(|(idx, item)| {
                    ErrorDescription::new(
                        "type",
                        format!(r#"{item} at index {idx} is not of type "boolean""#),
                    )
                })
                .collect();
            let schema_was_applied = !items.is_empty();
            if errors.is_empty() {
                let mut result = EvaluationResult::valid_empty();
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            } else {
                let mut result = EvaluationResult::invalid_empty(errors);
                result.annotate(Annotations::new(serde_json::json!(schema_was_applied)));
                result
            }
        } else {
            EvaluationResult::valid_empty()
        }
    }
}
