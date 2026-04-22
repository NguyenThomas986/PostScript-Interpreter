// boolean.rs — Boolean, comparison, type, and conversion operators
//
// Implements all boolean/comparison operators as methods on OperandStack.
//
// Operators:
//   eq, ne, ge, gt, le, lt    — comparison (original)
//   and, or, not              — logical / bitwise (original)
//   true, false               — boolean literal pushers (original)
//   type                      — push a name representing the runtime type
//   cvi, cvr, cvs, cvn        — type conversion operators
//
// PostScript rules:
//   - eq/ne work on any two values of the same type
//   - ge/gt/le/lt work on numbers or strings
//   - and/or/not work on booleans OR integers (bitwise)
//   - true/false push boolean literals onto the stack
//   - type pops a value and pushes a name like "integertype"
//   - cvi/cvr/cvs/cvn coerce between scalar types

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {
    pub fn op_eq(&mut self) -> Result<(), String> {
        let b = self.pop()?; let a = self.pop()?;
        self.push(Value::Bool(values_equal(&a, &b))); Ok(())
    }

    pub fn op_ne(&mut self) -> Result<(), String> {
        let b = self.pop()?; let a = self.pop()?;
        self.push(Value::Bool(!values_equal(&a, &b))); Ok(())
    }

    pub fn op_ge(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a >= b)); Ok(())
    }

    pub fn op_gt(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a > b)); Ok(())
    }

    pub fn op_le(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a <= b)); Ok(())
    }

    pub fn op_lt(&mut self) -> Result<(), String> {
        let (a, b) = self.pop_comparable()?;
        self.push(Value::Bool(a < b)); Ok(())
    }

    pub fn op_and(&mut self) -> Result<(), String> {
        let b = self.pop()?; let a = self.pop()?;
        match (a, b) {
            (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x && y)),
            (Value::Int(x),  Value::Int(y))  => self.push(Value::Int(x & y)),
            (a, b) => return Err(format!("and: incompatible types {:?} {:?}", a, b)),
        }
        Ok(())
    }

    pub fn op_or(&mut self) -> Result<(), String> {
        let b = self.pop()?; let a = self.pop()?;
        match (a, b) {
            (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x || y)),
            (Value::Int(x),  Value::Int(y))  => self.push(Value::Int(x | y)),
            (a, b) => return Err(format!("or: incompatible types {:?} {:?}", a, b)),
        }
        Ok(())
    }

    pub fn op_not(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Bool(b) => self.push(Value::Bool(!b)),
            Value::Int(n)  => self.push(Value::Int(!n)),
            other => return Err(format!("not: expected bool or int, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_true(&mut self)  -> Result<(), String> { self.push(Value::Bool(true));  Ok(()) }
    pub fn op_false(&mut self) -> Result<(), String> { self.push(Value::Bool(false)); Ok(()) }

    // ── Type and conversion operators ─────────────────────────────────────────

    /// type — pop a value, push a Name representing its runtime type.
    ///   42 type       → /integertype
    ///   3.14 type     → /realtype
    ///   (hi) type     → /stringtype
    ///   /foo type     → /nametype
    ///   true type     → /booleantype
    ///   { } type      → /proceduretype
    ///   5 dict type   → /dicttype
    ///   [ ] type      → /arraytype
    ///   mark type     → /marktype
    pub fn op_type(&mut self) -> Result<(), String> {
        let val = self.pop()?;
        let type_name = match &val {
            Value::Int(_)        => "integertype",
            Value::Float(_)      => "realtype",
            Value::Bool(_)       => "booleantype",
            Value::Str(_)        => "stringtype",
            Value::Name(_)       => "nametype",
            Value::Procedure {..} => "proceduretype",
            Value::Dict(_)       => "dicttype",
            Value::Array(_)      => "arraytype",
            Value::Mark          => "marktype",
        };
        self.push(Value::Name(type_name.to_string()));
        Ok(())
    }

    /// cvs — value string → string
    /// Convert any value to its string representation.
    /// PostScript passes a destination string buffer as the second argument;
    /// we accept (and discard) it for compatibility and always return a fresh string.
    ///   42 (     ) cvs  →  (42)
    pub fn op_cvs(&mut self) -> Result<(), String> {
        let _dest = self.pop()?; // destination buffer — ignored in this implementation
        let val = self.pop()?;
        let s = match &val {
            Value::Int(n)   => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b)  => b.to_string(),
            Value::Str(s)   => s.clone(),
            Value::Name(n)  => n.clone(),
            other => format!("{}", other),
        };
        self.push(Value::Str(s));
        Ok(())
    }

    /// cvi — value → int
    /// Convert to integer: floats are truncated toward zero, strings are parsed.
    ///   3.9 cvi  →  3
    ///   (7) cvi  →  7
    pub fn op_cvi(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n)),
            Value::Float(f) => self.push(Value::Int(f as i64)),
            Value::Str(s)   => {
                let n = s.trim().parse::<i64>()
                    .map_err(|_| format!("cvi: cannot convert '{}' to int", s))?;
                self.push(Value::Int(n));
            }
            other => return Err(format!("cvi: cannot convert {:?} to int", other)),
        }
        Ok(())
    }

    /// cvr — value → float
    /// Convert to real: integers are promoted, strings are parsed.
    ///   5 cvr     →  5.0
    ///   (3.14) cvr  →  3.14
    pub fn op_cvr(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Float(n as f64)),
            Value::Float(f) => self.push(Value::Float(f)),
            Value::Str(s)   => {
                let f = s.trim().parse::<f64>()
                    .map_err(|_| format!("cvr: cannot convert '{}' to real", s))?;
                self.push(Value::Float(f));
            }
            other => return Err(format!("cvr: cannot convert {:?} to real", other)),
        }
        Ok(())
    }

    /// cvn — string/name → name
    /// Convert a string to a name object. Names are like strings but are
    /// interned identifiers — used as dictionary keys.
    ///   (foo) cvn  →  /foo
    pub fn op_cvn(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Str(s)  => self.push(Value::Name(s)),
            Value::Name(n) => self.push(Value::Name(n)),
            other => return Err(format!("cvn: expected string or name, got {:?}", other)),
        }
        Ok(())
    }

    fn pop_comparable(&mut self) -> Result<(ComparableValue, ComparableValue), String> {
        let b = self.pop()?; let a = self.pop()?;
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

#[derive(PartialEq, PartialOrd)]
enum ComparableValue { Num(f64), Str(String) }

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x),   Value::Int(y))   => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x),   Value::Float(y)) => (*x as f64) == *y,
        (Value::Float(x), Value::Int(y))   => *x == (*y as f64),
        (Value::Bool(x),  Value::Bool(y))  => x == y,
        (Value::Str(x),   Value::Str(y))   => x == y,
        (Value::Name(x),  Value::Name(y))  => x == y,
        _ => false,
    }
}

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
        let mut s = stack_with(vec![Value::Int(3), Value::Float(3.0)]);
        s.op_eq().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_gt() {
        let mut s = stack_with(vec![Value::Int(5), Value::Int(3)]);
        s.op_gt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_and_bool() {
        let mut s = stack_with(vec![Value::Bool(true), Value::Bool(false)]);
        s.op_and().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_not_bool() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        s.op_not().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_type_int() {
        let mut s = stack_with(vec![Value::Int(42)]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("integertype".to_string()));
    }

    #[test]
    fn test_type_string() {
        let mut s = stack_with(vec![Value::Str("hello".to_string())]);
        s.op_type().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("stringtype".to_string()));
    }

    #[test]
    fn test_cvi_float() {
        let mut s = stack_with(vec![Value::Float(3.9)]);
        s.op_cvi().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(3));
    }

    #[test]
    fn test_cvr_int() {
        let mut s = stack_with(vec![Value::Int(5)]);
        s.op_cvr().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(5.0));
    }

    #[test]
    fn test_cvn_string() {
        let mut s = stack_with(vec![Value::Str("foo".to_string())]);
        s.op_cvn().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Name("foo".to_string()));
    }
}