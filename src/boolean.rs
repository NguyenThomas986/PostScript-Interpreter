// boolean.rs — Boolean, comparison, type, and conversion operators

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {
    pub fn op_eq(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(Value::Bool(values_equal(&a, &b)));
        Ok(())
    }

    pub fn op_ne(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(Value::Bool(!values_equal(&a, &b)));
        Ok(())
    }

    pub fn op_ge(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a >= b));
        Ok(())
    }

    pub fn op_gt(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a > b));
        Ok(())
    }

    pub fn op_le(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a <= b));
        Ok(())
    }

    pub fn op_lt(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a < b));
        Ok(())
    }

    pub fn op_and(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        match (a, b) {
            (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x && y)),
            (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x & y)),
            (a, b) => return Err(format!("and: incompatible types {:?} {:?}", a, b)),
        }
        Ok(())
    }

    pub fn op_or(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        match (a, b) {
            (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x || y)),
            (Value::Int(x), Value::Int(y)) => self.push(Value::Int(x | y)),
            (a, b) => return Err(format!("or: incompatible types {:?} {:?}", a, b)),
        }
        Ok(())
    }

    pub fn op_not(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Bool(b) => self.push(Value::Bool(!b)),
            Value::Int(n) => self.push(Value::Int(!n)),
            other => return Err(format!("not: expected bool or int, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_true(&mut self) -> Result<(), String> {
        self.push(Value::Bool(true));
        Ok(())
    }
    pub fn op_false(&mut self) -> Result<(), String> {
        self.push(Value::Bool(false));
        Ok(())
    }

    pub fn op_type(&mut self) -> Result<(), String> {
        let val = self.pop()?;
        let type_name = match &val {
            Value::Int(_) => "integertype",
            Value::Float(_) => "realtype",
            Value::Bool(_) => "booleantype",
            Value::Str(_) => "stringtype",
            Value::Name(_) => "nametype",
            Value::Procedure { .. } => "proceduretype",
            Value::Dict(_) => "dicttype",
            Value::Array(_) => "arraytype",
            Value::Mark => "marktype",
        };
        self.push(Value::Name(type_name.to_string()));
        Ok(())
    }

    pub fn op_cvs(&mut self) -> Result<(), String> {
        let _dest = self.pop()?;
        let val = self.pop()?;
        let s = match &val {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.clone(),
            Value::Name(n) => n.clone(),
            other => format!("{}", other),
        };
        self.push(Value::Str(s));
        Ok(())
    }

    pub fn op_cvi(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n) => self.push(Value::Int(n)),
            Value::Float(f) => self.push(Value::Int(f as i64)),
            Value::Str(s) => {
                let n = s
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| format!("cvi: cannot convert '{}' to int", s))?;
                self.push(Value::Int(n));
            }
            other => return Err(format!("cvi: cannot convert {:?} to int", other)),
        }
        Ok(())
    }

    pub fn op_cvr(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n) => self.push(Value::Float(n as f64)),
            Value::Float(f) => self.push(Value::Float(f)),
            Value::Str(s) => {
                let f = s
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| format!("cvr: cannot convert '{}' to real", s))?;
                self.push(Value::Float(f));
            }
            other => return Err(format!("cvr: cannot convert {:?} to real", other)),
        }
        Ok(())
    }

    pub fn op_cvn(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Str(s) => self.push(Value::Name(s)),
            Value::Name(n) => self.push(Value::Name(n)),
            other => return Err(format!("cvn: expected string or name, got {:?}", other)),
        }
        Ok(())
    }

    fn pop_comparable(&mut self) -> Result<(ComparableValue, ComparableValue), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok((
                ComparableValue::Num(x as f64),
                ComparableValue::Num(y as f64),
            )),
            (Value::Float(x), Value::Float(y)) => {
                Ok((ComparableValue::Num(x), ComparableValue::Num(y)))
            }
            (Value::Int(x), Value::Float(y)) => {
                Ok((ComparableValue::Num(x as f64), ComparableValue::Num(y)))
            }
            (Value::Float(x), Value::Int(y)) => {
                Ok((ComparableValue::Num(x), ComparableValue::Num(y as f64)))
            }
            (Value::Str(x), Value::Str(y)) => {
                Ok((ComparableValue::Str(x), ComparableValue::Str(y)))
            }
            (a, b) => Err(format!("comparison: incompatible types {:?} {:?}", a, b)),
        }
    }
}

#[derive(PartialEq, PartialOrd)]
enum ComparableValue {
    Num(f64),
    Str(String),
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
        (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Name(x), Value::Name(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack_with(vals: Vec<Value>) -> OperandStack {
        let mut s = OperandStack::new();
        for v in vals {
            s.push(v);
        }
        s
    }

    // ── eq / ne ──────────────────────────────────────────────────────────────

    #[test]
    fn test_eq_true() {
        let mut s = stack_with(vec![Value::Int(3), Value::Int(3)]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eq_false() {
        let mut s = stack_with(vec![Value::Int(3), Value::Int(4)]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_eq_int_float() {
        let mut s = stack_with(vec![Value::Int(3), Value::Float(3.0)]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eq_float_int() {
        let mut s = stack_with(vec![Value::Float(3.0), Value::Int(3)]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eq_bools() {
        let mut s = stack_with(vec![Value::Bool(true), Value::Bool(true)]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eq_strings() {
        let mut s = stack_with(vec![
            Value::Str("hi".to_string()),
            Value::Str("hi".to_string()),
        ]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eq_names() {
        let mut s = stack_with(vec![
            Value::Name("foo".to_string()),
            Value::Name("foo".to_string()),
        ]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eq_different_types() {
        let mut s = stack_with(vec![Value::Int(1), Value::Bool(true)]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_ne() {
        let mut s = stack_with(vec![Value::Int(1), Value::Int(2)]);
        s.op_ne().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    // ── comparisons ──────────────────────────────────────────────────────────

    #[test]
    fn test_gt() {
        let mut s = stack_with(vec![Value::Int(5), Value::Int(3)]);
        s.op_gt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_ge() {
        let mut s = stack_with(vec![Value::Int(3), Value::Int(3)]);
        s.op_ge().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_lt() {
        let mut s = stack_with(vec![Value::Int(2), Value::Int(5)]);
        s.op_lt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_le() {
        let mut s = stack_with(vec![Value::Int(3), Value::Int(3)]);
        s.op_le().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_compare_floats() {
        let mut s = stack_with(vec![Value::Float(1.5), Value::Float(2.5)]);
        s.op_lt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_compare_int_float() {
        let mut s = stack_with(vec![Value::Int(1), Value::Float(2.0)]);
        s.op_lt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_compare_float_int() {
        let mut s = stack_with(vec![Value::Float(1.0), Value::Int(2)]);
        s.op_lt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_compare_strings() {
        let mut s = stack_with(vec![
            Value::Str("abc".to_string()),
            Value::Str("abd".to_string()),
        ]);
        s.op_lt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_compare_incompatible_types() {
        let mut s = stack_with(vec![Value::Int(1), Value::Str("hi".to_string())]);
        assert!(s.op_lt().is_err());
    }

    // ── and / or / not ────────────────────────────────────────────────────────

    #[test]
    fn test_and_bool() {
        let mut s = stack_with(vec![Value::Bool(true), Value::Bool(false)]);
        s.op_and().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_and_int_bitwise() {
        let mut s = stack_with(vec![Value::Int(0b1100), Value::Int(0b1010)]);
        s.op_and().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(0b1000));
    }

    #[test]
    fn test_and_incompatible() {
        let mut s = stack_with(vec![Value::Bool(true), Value::Int(1)]);
        assert!(s.op_and().is_err());
    }

    #[test]
    fn test_or_bool() {
        let mut s = stack_with(vec![Value::Bool(false), Value::Bool(true)]);
        s.op_or().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_or_int_bitwise() {
        let mut s = stack_with(vec![Value::Int(0b1100), Value::Int(0b0011)]);
        s.op_or().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(0b1111));
    }

    #[test]
    fn test_or_incompatible() {
        let mut s = stack_with(vec![Value::Bool(true), Value::Int(1)]);
        assert!(s.op_or().is_err());
    }

    #[test]
    fn test_not_bool() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        s.op_not().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_not_int_bitwise() {
        let mut s = stack_with(vec![Value::Int(0)]);
        s.op_not().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(-1));
    }

    #[test]
    fn test_not_wrong_type() {
        let mut s = stack_with(vec![Value::Str("hi".to_string())]);
        assert!(s.op_not().is_err());
    }

    #[test]
    fn test_true_false() {
        let mut s = OperandStack::new();
        s.op_true().unwrap();
        s.op_false().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    // ── type ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_type_int() {
        let mut s = stack_with(vec![Value::Int(42)]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("integertype".to_string()));
    }

    #[test]
    fn test_type_float() {
        let mut s = stack_with(vec![Value::Float(3.14)]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("realtype".to_string()));
    }

    #[test]
    fn test_type_bool() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("booleantype".to_string()));
    }

    #[test]
    fn test_type_string() {
        let mut s = stack_with(vec![Value::Str("hello".to_string())]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("stringtype".to_string()));
    }

    #[test]
    fn test_type_name() {
        let mut s = stack_with(vec![Value::Name("foo".to_string())]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("nametype".to_string()));
    }

    #[test]
    fn test_type_array() {
        let mut s = stack_with(vec![Value::Array(vec![])]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("arraytype".to_string()));
    }

    #[test]
    fn test_type_mark() {
        let mut s = stack_with(vec![Value::Mark]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("marktype".to_string()));
    }

    // ── cvs ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_cvs_int() {
        let mut s = stack_with(vec![Value::Int(42), Value::Str(String::new())]);
        s.op_cvs().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("42".to_string()));
    }

    #[test]
    fn test_cvs_float() {
        let mut s = stack_with(vec![Value::Float(3.14), Value::Str(String::new())]);
        s.op_cvs().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("3.14".to_string()));
    }

    #[test]
    fn test_cvs_bool() {
        let mut s = stack_with(vec![Value::Bool(true), Value::Str(String::new())]);
        s.op_cvs().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("true".to_string()));
    }

    #[test]
    fn test_cvs_string_passthrough() {
        let mut s = stack_with(vec![
            Value::Str("hi".to_string()),
            Value::Str(String::new()),
        ]);
        s.op_cvs().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("hi".to_string()));
    }

    #[test]
    fn test_cvs_name() {
        let mut s = stack_with(vec![
            Value::Name("foo".to_string()),
            Value::Str(String::new()),
        ]);
        s.op_cvs().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("foo".to_string()));
    }

    // ── cvi ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_cvi_int_passthrough() {
        let mut s = stack_with(vec![Value::Int(7)]);
        s.op_cvi().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(7));
    }

    #[test]
    fn test_cvi_float() {
        let mut s = stack_with(vec![Value::Float(3.9)]);
        s.op_cvi().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(3));
    }

    #[test]
    fn test_cvi_string() {
        let mut s = stack_with(vec![Value::Str("42".to_string())]);
        s.op_cvi().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(42));
    }

    #[test]
    fn test_cvi_string_invalid() {
        let mut s = stack_with(vec![Value::Str("abc".to_string())]);
        assert!(s.op_cvi().is_err());
    }

    #[test]
    fn test_cvi_wrong_type() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        assert!(s.op_cvi().is_err());
    }

    // ── cvr ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_cvr_int() {
        let mut s = stack_with(vec![Value::Int(5)]);
        s.op_cvr().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(5.0));
    }

    #[test]
    fn test_cvr_float_passthrough() {
        let mut s = stack_with(vec![Value::Float(2.5)]);
        s.op_cvr().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(2.5));
    }

    #[test]
    fn test_cvr_string() {
        let mut s = stack_with(vec![Value::Str("3.14".to_string())]);
        s.op_cvr().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(3.14));
    }

    #[test]
    fn test_cvr_string_invalid() {
        let mut s = stack_with(vec![Value::Str("abc".to_string())]);
        assert!(s.op_cvr().is_err());
    }

    #[test]
    fn test_cvr_wrong_type() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        assert!(s.op_cvr().is_err());
    }

    // ── cvn ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_cvn_string() {
        let mut s = stack_with(vec![Value::Str("foo".to_string())]);
        s.op_cvn().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("foo".to_string()));
    }

    #[test]
    fn test_cvn_name_passthrough() {
        let mut s = stack_with(vec![Value::Name("bar".to_string())]);
        s.op_cvn().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("bar".to_string()));
    }

    #[test]
    fn test_cvn_wrong_type() {
        let mut s = stack_with(vec![Value::Int(1)]);
        assert!(s.op_cvn().is_err());
    }
}
