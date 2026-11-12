use super::*;

impl Validate for MultipleOfFloatValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Number(item) = instance {
            numeric::is_multiple_of_float(item, self.multiple_of)
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
        if !self.is_valid(instance, ctx) {
            #[cfg(feature = "arbitrary-precision")]
            {
                return Err(ValidationError::multiple_of(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    Value::Number(self.original_value.clone()),
                ));
            }
            #[cfg(not(feature = "arbitrary-precision"))]
            {
                return Err(ValidationError::multiple_of(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    self.multiple_of,
                ));
            }
        }
        Ok(())
    }
}

impl Validate for MultipleOfIntegerValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        if let Value::Number(item) = instance {
            numeric::is_multiple_of_integer(item, self.multiple_of)
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
        if !self.is_valid(instance, ctx) {
            #[cfg(feature = "arbitrary-precision")]
            {
                return Err(ValidationError::multiple_of(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    Value::Number(self.original_value.clone()),
                ));
            }
            #[cfg(not(feature = "arbitrary-precision"))]
            {
                return Err(ValidationError::multiple_of(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance,
                    self.multiple_of,
                ));
            }
        }
        Ok(())
    }
}

#[cfg(feature = "arbitrary-precision")]
impl Validate for MultipleOfBigIntValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        use num_bigint::BigInt;
        use num_traits::One;
        if let Value::Number(item) = instance {
            if let Some(instance_bigint) = numeric::bignum::try_parse_bigint(item) {
                numeric::bignum::is_multiple_of_bigint(&instance_bigint, &self.multiple_of)
            } else if let Some(v) = item.as_u64() {
                let v_bigint = BigInt::from(v);
                numeric::bignum::is_multiple_of_bigint(&v_bigint, &self.multiple_of)
            } else if let Some(v) = item.as_i64() {
                let v_bigint = BigInt::from(v);
                numeric::bignum::is_multiple_of_bigint(&v_bigint, &self.multiple_of)
            } else if let Some(v) = item.as_f64() {
                if v.fract() != 0.0 {
                    return false;
                }
                // JSON numbers that originate as f64 are only exactly representable if their
                // absolute value is below 2^53. Within that "safe integer" window we can round-trip
                // to i64 without losing bits, so convert and reuse the bigint path.
                #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
                if v.is_finite() && v.abs() < (1u64 << 53) as f64 {
                    let v_bigint = BigInt::from(v as i64);
                    numeric::bignum::is_multiple_of_bigint(&v_bigint, &self.multiple_of)
                } else {
                    // 2^53 and above can no longer be recovered exactly from the binary64 mantissa,
                    // so we bail out instead of pretending we know whether the bigint divisor divides
                    // such an approximation.
                    false
                }
            } else {
                // as_f64() returning None means the literal overflowed binary64 (e.g. 1e10000) or was
                // written in plain decimal form with more precision than f64 can carry. We attempt to
                // parse it as an exact BigFraction and only accept it when the denominator is 1 so we
                // can safely convert it to BigInt before running the modulo check below.
                if let Some(instance_bigfrac) = numeric::bignum::try_parse_bigfraction(item) {
                    if instance_bigfrac.denom().is_none_or(One::is_one) {
                        if let Some(numer) = instance_bigfrac.numer() {
                            let instance_bigint = BigInt::from(numer.clone());
                            return numeric::bignum::is_multiple_of_bigint(
                                &instance_bigint,
                                &self.multiple_of,
                            );
                        }
                    }
                }
                // If we made it here we encountered scientific notation that can’t be normalized, or
                // a decimal with a fractional part. Either way it cannot be an integer multiple.
                false
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
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if !self.is_valid(instance, ctx) {
            return Err(ValidationError::multiple_of(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                Value::Number(self.original_value.clone()),
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "arbitrary-precision")]
impl Validate for MultipleOfBigFracValidator {
    fn is_valid(&self, instance: &Value, _ctx: &mut ValidationContext) -> bool {
        use num_traits::ToPrimitive;
        if let Value::Number(item) = instance {
            // Try to parse instance as BigFraction for exact precision
            if let Some(instance_bigfrac) = numeric::bignum::try_parse_bigfraction(item) {
                numeric::bignum::is_multiple_of_bigfrac(&instance_bigfrac, &self.multiple_of)
            } else if let Some(instance_bigint) = numeric::bignum::try_parse_bigint(item) {
                let value_frac = fraction::BigFraction::from(instance_bigint);
                numeric::bignum::is_multiple_of_bigfrac(&value_frac, &self.multiple_of)
            } else if let Some(v) = item.as_u64() {
                let v_frac = fraction::BigFraction::from(v);
                numeric::bignum::is_multiple_of_bigfrac(&v_frac, &self.multiple_of)
            } else if let Some(v) = item.as_i64() {
                let v_frac = fraction::BigFraction::from(v);
                numeric::bignum::is_multiple_of_bigfrac(&v_frac, &self.multiple_of)
            } else {
                // Fall back to the regular float-based validation since precision is already lost
                let multiple_f64 = self.multiple_of.to_f64().unwrap_or(f64::INFINITY);
                numeric::is_multiple_of_float(item, multiple_f64)
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
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if !self.is_valid(instance, ctx) {
            return Err(ValidationError::multiple_of(
                self.location.clone(),
                crate::paths::capture_evaluation_path(tracker, &self.location),
                location.into(),
                instance,
                Value::Number(self.original_value.clone()),
            ));
        }
        Ok(())
    }
}
