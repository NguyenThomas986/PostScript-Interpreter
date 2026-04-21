// boolean.rs — Boolean, comparison, and bitwise operators
//
// Implements all boolean/comparison operators as methods on OperandStack.
//
// Operators: eq, ne, ge, gt, le, lt, and, or, not, true, false
//
// PostScript rules:
//   - eq/ne work on any two values of the same type
//   - ge/gt/le/lt work on numbers or strings
//   - and/or/not work on booleans OR integers (bitwise)
//   - true/false push boolean literals onto the stack

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {

    // ── Equality ──────────────────────────────────────────────────────────────

    /// eq — push true if top two values are equal, false otherwise
    ///   Before: a b    After: bool
    pub fn op_eq(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(Value::Bool(values_equal(&a, &b)));
        Ok(())
    }

    /// ne — push true if top two values are NOT equal
    ///   Before: a b    After: bool
    pub fn op_ne(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(Value::Bool(!values_equal(&a, &b)));
        Ok(())
    }

    // ── Ordering comparisons ──────────────────────────────────────────────────

    /// ge — push true if a >= b
    pub fn op_ge(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a >= b));
        Ok(())
    }

    /// gt — push true if a > b
    pub fn op_gt(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a > b));
        Ok(())
    }

    /// le — push true if a <= b
    pub fn op_le(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a <= b));
        Ok(())
    }

    /// lt — push true if a < b
    pub fn op_lt(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a < b));
        Ok(())
    }

    // ── Logical / bitwise operators ───────────────────────────────────────────

    /// and — bool bool → bool  (logical)  OR  int int → int  (bitwise)
    pub fn op_and(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        match (a, b) {
            (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x && y)),
            (Value::Int(x),  Value::Int(y))  => self.push(Value::Int(x & y)),
            (a, b) => return Err(format!("and: incompatible types {:?} {:?}", a, b)),
        }
        Ok(())
    }

    /// or — bool bool → bool  (logical)  OR  int int → int  (bitwise)
    pub fn op_or(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        match (a, b) {
            (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x || y)),
            (Value::Int(x),  Value::Int(y))  => self.push(Value::Int(x | y)),
            (a, b) => return Err(format!("or: incompatible types {:?} {:?}", a, b)),
        }
        Ok(())
    }

    /// not — bool → bool  (logical)  OR  int → int  (bitwise)
    pub fn op_not(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Bool(b) => self.push(Value::Bool(!b)),
            Value::Int(n)  => self.push(Value::Int(!n)),
            other => return Err(format!("not: expected bool or int, got {:?}", other)),
        }
        Ok(())
    }

    /// true — push the boolean literal true
    pub fn op_true(&mut self) -> Result<(), String> {
        self.push(Value::Bool(true));
        Ok(())
    }

    /// false — push the boolean literal false
    pub fn op_false(&mut self) -> Result<(), String> {
        self.push(Value::Bool(false));
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Pop two values and return them as an ordered (f64, f64) pair for
    /// numeric comparisons, or compare strings lexicographically.
    /// Returns (a_ord, b_ord) as f64 for numbers, errors on incompatible types.
    fn pop_comparable(&mut self) -> Result<(ComparableValue, ComparableValue), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        match (a, b) {
            (Value::Int(x),   Value::Int(y))   => Ok((ComparableValue::Num(x as f64), ComparableValue::Num(y as f64))),
            (Value::Float(x), Value::Float(y)) => Ok((ComparableValue::Num(x),        ComparableValue::Num(y))),
            (Value::Int(x),   Value::Float(y)) => Ok((ComparableValue::Num(x as f64), ComparableValue::Num(y))),
            (Value::Float(x), Value::Int(y))   => Ok((ComparableValue::Num(x),        ComparableValue::Num(y as f64))),
            (Value::Str(x),   Value::Str(y))   => Ok((ComparableValue::Str(x),        ComparableValue::Str(y))),
            (a, b) => Err(format!("comparison: incompatible types {:?} {:?}", a, b)),
        }
    }
}

/// Internal helper type for ordering comparisons.
/// Lets us compare numbers as f64 and strings lexicographically
/// without duplicating match arms for every operator.
#[derive(PartialEq, PartialOrd)]
enum ComparableValue {
    Num(f64),
    Str(String),
}

/// Check deep equality between two Values.
/// Handles int/float cross-type equality (3 == 3.0 → true).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x),   Value::Int(y))   => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        // Cross-type numeric equality: 3 eq 3.0 → true
        (Value::Int(x),   Value::Float(y)) => (*x as f64) == *y,
        (Value::Float(x), Value::Int(y))   => *x == (*y as f64),
        (Value::Bool(x),  Value::Bool(y))  => x == y,
        (Value::Str(x),   Value::Str(y))   => x == y,
        (Value::Name(x),  Value::Name(y))  => x == y,
        _ => false,
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn stack_with(vals: Vec<Value>) -> OperandStack {
        let mut s = OperandStack::new();
        for v in vals { s.push(v); }
        s
    }

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
        // Cross-type: 3 eq 3.0 should be true
        let mut s = stack_with(vec![Value::Int(3), Value::Float(3.0)]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_ne() {
        let mut s = stack_with(vec![Value::Int(1), Value::Int(2)]);
        s.op_ne().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_gt() {
        let mut s = stack_with(vec![Value::Int(5), Value::Int(3)]);
        s.op_gt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_lt() {
        let mut s = stack_with(vec![Value::Int(2), Value::Int(9)]);
        s.op_lt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_ge_equal() {
        let mut s = stack_with(vec![Value::Int(4), Value::Int(4)]);
        s.op_ge().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_le() {
        let mut s = stack_with(vec![Value::Int(3), Value::Int(5)]);
        s.op_le().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_and_bool() {
        let mut s = stack_with(vec![Value::Bool(true), Value::Bool(false)]);
        s.op_and().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_or_bool() {
        let mut s = stack_with(vec![Value::Bool(false), Value::Bool(true)]);
        s.op_or().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_not_bool() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        s.op_not().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_and_bitwise() {
        // 0b1100 & 0b1010 = 0b1000 = 8
        let mut s = stack_with(vec![Value::Int(12), Value::Int(10)]);
        s.op_and().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(8));
    }

    #[test]
    fn test_or_bitwise() {
        // 0b1100 | 0b1010 = 0b1110 = 14
        let mut s = stack_with(vec![Value::Int(12), Value::Int(10)]);
        s.op_or().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(14));
    }

    #[test]
    fn test_true_false_push() {
        let mut s = OperandStack::new();
        s.op_true().unwrap();
        s.op_false().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_string_comparison() {
        let mut s = stack_with(vec![Value::Str("abc".to_string()), Value::Str("abd".to_string())]);
        s.op_lt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }
}