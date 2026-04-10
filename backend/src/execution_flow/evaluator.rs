use std::fmt;

/// Represents a runtime value during execution flow analysis.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Null,
    Unknown,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => {
                if v.fract() == 0.0 {
                    write!(f, "{v:.1}")
                } else {
                    write!(f, "{v}")
                }
            }
            Value::Bool(v) => write!(f, "{v}"),
            Value::Str(v) => write!(f, "\"{v}\""),
            Value::Null => write!(f, "null"),
            Value::Unknown => write!(f, "?"),
        }
    }
}

impl Value {
    /// Try to interpret as boolean. Int != 0 is true, Bool is direct, everything else None.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Int(i) => Some(*i != 0),
            _ => None,
        }
    }

    /// Try to interpret as f64. Int converts, Float direct, everything else None.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to interpret as i64. Int direct, Float if it's a whole number, everything else None.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) if f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        }
    }

    /// Check if value is known (not Unknown).
    pub fn is_known(&self) -> bool {
        !matches!(self, Value::Unknown)
    }

    /// Convert to string representation suitable for variable storage.
    pub fn to_storage_string(&self) -> String {
        match self {
            Value::Int(v) => format!("{v}"),
            Value::Float(v) => {
                if v.floor() == *v {
                    format!("{v:.1}")
                } else {
                    format!("{v}")
                }
            }
            Value::Bool(v) => format!("{v}"),
            Value::Str(v) => v.clone(),
            Value::Null => "null".to_string(),
            Value::Unknown => "unknown".to_string(),
        }
    }

    // -- Arithmetic --

    pub fn add(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_add(*b)),
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Float(fa + fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn sub(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_sub(*b)),
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Float(fa - fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn mul(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_mul(*b)),
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Float(fa * fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn div(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Value::Unknown
                } else {
                    Value::Int(a / b)
                }
            }
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(_), Some(fb)) if fb == 0.0 => Value::Unknown,
                (Some(fa), Some(fb)) => Value::Float(fa / fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn rem(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Value::Unknown
                } else {
                    Value::Int(a % b)
                }
            }
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(_), Some(fb)) if fb == 0.0 => Value::Unknown,
                (Some(fa), Some(fb)) => Value::Float(fa % fb),
                _ => Value::Unknown,
            },
        }
    }

    // -- Comparison --

    pub fn lt(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Bool(fa < fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn le(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Bool(fa <= fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn gt(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Bool(fa > fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn ge(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Bool(fa >= fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn eq_val(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (Value::Bool(a), Value::Bool(b)) => Value::Bool(a == b),
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Bool(fa == fb),
                _ => Value::Unknown,
            },
        }
    }

    pub fn ne_val(&self, other: &Value) -> Value {
        match (self, other) {
            (Value::Unknown, _) | (_, Value::Unknown) => Value::Unknown,
            (Value::Bool(a), Value::Bool(b)) => Value::Bool(a != b),
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => Value::Bool(fa != fb),
                _ => Value::Unknown,
            },
        }
    }

    // -- Boolean --

    pub fn and(&self, other: &Value) -> Value {
        // Short-circuit: false && anything = false
        if let Some(false) = self.as_bool() {
            return Value::Bool(false);
        }
        match (self.as_bool(), other.as_bool()) {
            (Some(a), Some(b)) => Value::Bool(a && b),
            _ => Value::Unknown,
        }
    }

    pub fn or(&self, other: &Value) -> Value {
        // Short-circuit: true || anything = true
        if let Some(true) = self.as_bool() {
            return Value::Bool(true);
        }
        match (self.as_bool(), other.as_bool()) {
            (Some(a), Some(b)) => Value::Bool(a || b),
            _ => Value::Unknown,
        }
    }

    pub fn not(&self) -> Value {
        match self.as_bool() {
            Some(b) => Value::Bool(!b),
            None => Value::Unknown,
        }
    }

    // -- Unary --

    pub fn negate(&self) -> Value {
        match self {
            Value::Int(v) => Value::Int(-v),
            Value::Float(v) => Value::Float(-v),
            _ => Value::Unknown,
        }
    }
}

/// Signal emitted by flow-control statements (break, continue).
#[derive(Debug, Clone, PartialEq)]
pub enum FlowSignal {
    Break,
    Continue,
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Arithmetic --

    #[test]
    fn int_add() {
        assert_eq!(Value::Int(3).add(&Value::Int(4)), Value::Int(7));
    }

    #[test]
    fn int_sub() {
        assert_eq!(Value::Int(10).sub(&Value::Int(3)), Value::Int(7));
    }

    #[test]
    fn int_mul() {
        assert_eq!(Value::Int(6).mul(&Value::Int(7)), Value::Int(42));
    }

    #[test]
    fn int_div_truncates() {
        assert_eq!(Value::Int(7).div(&Value::Int(2)), Value::Int(3));
    }

    #[test]
    fn int_rem() {
        assert_eq!(Value::Int(7).rem(&Value::Int(3)), Value::Int(1));
    }

    #[test]
    fn float_promotion_add() {
        assert_eq!(Value::Int(3).add(&Value::Float(1.5)), Value::Float(4.5));
    }

    #[test]
    fn float_arithmetic() {
        assert_eq!(
            Value::Float(2.5).mul(&Value::Float(4.0)),
            Value::Float(10.0)
        );
    }

    #[test]
    fn div_by_zero_int() {
        assert_eq!(Value::Int(5).div(&Value::Int(0)), Value::Unknown);
    }

    #[test]
    fn div_by_zero_float() {
        assert_eq!(Value::Float(5.0).div(&Value::Float(0.0)), Value::Unknown);
    }

    // -- Comparison --

    #[test]
    fn gt_true() {
        assert_eq!(Value::Int(5).gt(&Value::Int(3)), Value::Bool(true));
    }

    #[test]
    fn gt_false() {
        assert_eq!(Value::Int(2).gt(&Value::Int(5)), Value::Bool(false));
    }

    #[test]
    fn lt_float() {
        assert_eq!(Value::Float(1.5).lt(&Value::Float(2.0)), Value::Bool(true));
    }

    #[test]
    fn eq_val_bools() {
        assert_eq!(
            Value::Bool(true).eq_val(&Value::Bool(true)),
            Value::Bool(true)
        );
        assert_eq!(
            Value::Bool(true).eq_val(&Value::Bool(false)),
            Value::Bool(false)
        );
    }

    #[test]
    fn ne_val_ints() {
        assert_eq!(Value::Int(1).ne_val(&Value::Int(2)), Value::Bool(true));
        assert_eq!(Value::Int(3).ne_val(&Value::Int(3)), Value::Bool(false));
    }

    // -- Boolean --

    #[test]
    fn and_basic() {
        assert_eq!(
            Value::Bool(true).and(&Value::Bool(false)),
            Value::Bool(false)
        );
        assert_eq!(Value::Bool(true).and(&Value::Bool(true)), Value::Bool(true));
    }

    #[test]
    fn or_basic() {
        assert_eq!(Value::Bool(true).or(&Value::Bool(false)), Value::Bool(true));
        assert_eq!(
            Value::Bool(false).or(&Value::Bool(false)),
            Value::Bool(false)
        );
    }

    #[test]
    fn not_basic() {
        assert_eq!(Value::Bool(true).not(), Value::Bool(false));
        assert_eq!(Value::Bool(false).not(), Value::Bool(true));
    }

    #[test]
    fn short_circuit_and() {
        // false && Unknown should be Bool(false), not Unknown
        assert_eq!(Value::Bool(false).and(&Value::Unknown), Value::Bool(false));
    }

    #[test]
    fn short_circuit_or() {
        // true || Unknown should be Bool(true), not Unknown
        assert_eq!(Value::Bool(true).or(&Value::Unknown), Value::Bool(true));
    }

    // -- as_bool --

    #[test]
    fn as_bool_int() {
        assert_eq!(Value::Int(0).as_bool(), Some(false));
        assert_eq!(Value::Int(1).as_bool(), Some(true));
        assert_eq!(Value::Int(-5).as_bool(), Some(true));
    }

    #[test]
    fn as_bool_bool() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Bool(false).as_bool(), Some(false));
    }

    // -- to_storage_string --

    #[test]
    fn storage_string_int() {
        assert_eq!(Value::Int(5).to_storage_string(), "5");
    }

    #[test]
    fn storage_string_float() {
        assert_eq!(Value::Float(5.0).to_storage_string(), "5.0");
        assert_eq!(Value::Float(3.14).to_storage_string(), "3.14");
    }

    #[test]
    fn storage_string_bool() {
        assert_eq!(Value::Bool(true).to_storage_string(), "true");
        assert_eq!(Value::Bool(false).to_storage_string(), "false");
    }

    #[test]
    fn storage_string_str() {
        assert_eq!(Value::Str("hello".into()).to_storage_string(), "hello");
    }

    #[test]
    fn storage_string_null_unknown() {
        assert_eq!(Value::Null.to_storage_string(), "null");
        assert_eq!(Value::Unknown.to_storage_string(), "unknown");
    }

    // -- Display --

    #[test]
    fn display_values() {
        assert_eq!(format!("{}", Value::Int(42)), "42");
        assert_eq!(format!("{}", Value::Float(5.0)), "5.0");
        assert_eq!(format!("{}", Value::Float(3.14)), "3.14");
        assert_eq!(format!("{}", Value::Bool(true)), "true");
        assert_eq!(format!("{}", Value::Str("hi".into())), "\"hi\"");
        assert_eq!(format!("{}", Value::Null), "null");
        assert_eq!(format!("{}", Value::Unknown), "?");
    }

    // -- Negate --

    #[test]
    fn negate_int() {
        assert_eq!(Value::Int(5).negate(), Value::Int(-5));
    }

    #[test]
    fn negate_float() {
        assert_eq!(Value::Float(3.14).negate(), Value::Float(-3.14));
    }

    #[test]
    fn negate_unknown() {
        assert_eq!(Value::Bool(true).negate(), Value::Unknown);
    }

    // -- Unknown propagation --

    #[test]
    fn unknown_propagation() {
        assert_eq!(Value::Unknown.add(&Value::Int(5)), Value::Unknown);
        assert_eq!(Value::Int(5).sub(&Value::Unknown), Value::Unknown);
        assert_eq!(Value::Unknown.gt(&Value::Int(3)), Value::Unknown);
        assert_eq!(Value::Unknown.not(), Value::Unknown);
    }

    // -- is_known --

    #[test]
    fn is_known_check() {
        assert!(Value::Int(0).is_known());
        assert!(Value::Null.is_known());
        assert!(!Value::Unknown.is_known());
    }

    // -- as_i64 / as_f64 --

    #[test]
    fn as_i64_from_float() {
        assert_eq!(Value::Float(5.0).as_i64(), Some(5));
        assert_eq!(Value::Float(5.5).as_i64(), None);
    }

    #[test]
    fn as_f64_from_int() {
        assert_eq!(Value::Int(3).as_f64(), Some(3.0));
    }
}
