// arithmetic.rs — Arithmetic operators
//
// Implements all math operators as methods on OperandStack.
// Separated here so interpreter.rs stays focused on dispatch and control flow.
//
// Operators: add, sub, mul, div, idiv, mod, abs, neg, ceiling, floor, round, sqrt

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {
    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Pop two numeric values. Returns (a, b, is_float) where is_float is true
    /// if either operand was a Float — used to decide the result type.
    /// PostScript rule: Int op Int → Int, any Float involved → Float.
    fn pop_two_numeric(&mut self) -> Result<(f64, f64, bool), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        let is_float = matches!((&a, &b), (Value::Float(_), _) | (_, Value::Float(_)));
        let to_f64 = |v: Value| -> Result<f64, String> {
            match v {
                Value::Int(n)   => Ok(n as f64),
                Value::Float(f) => Ok(f),
                other => Err(format!("Expected number, got {:?}", other)),
            }
        };
        Ok((to_f64(a)?, to_f64(b)?, is_float))
    }

    /// Pop one numeric value as f64
    fn pop_numeric(&mut self) -> Result<f64, String> {
        match self.pop()? {
            Value::Int(n)   => Ok(n as f64),
            Value::Float(f) => Ok(f),
            other => Err(format!("Expected number, got {:?}", other)),
        }
    }

    /// Push a numeric result as Int or Float based on the is_float flag
    fn push_numeric(&mut self, value: f64, is_float: bool) {
        if is_float {
            self.push(Value::Float(value));
        } else {
            self.push(Value::Int(value as i64));
        }
    }

    // ── Binary operators ──────────────────────────────────────────────────────

    /// add — num1 num2 → num1+num2
    pub fn op_add(&mut self) -> Result<(), String> {
        let (a, b, is_float) = self.pop_two_numeric()?;
        self.push_numeric(a + b, is_float);
        Ok(())
    }

    /// sub — num1 num2 → num1-num2
    pub fn op_sub(&mut self) -> Result<(), String> {
        let (a, b, is_float) = self.pop_two_numeric()?;
        self.push_numeric(a - b, is_float);
        Ok(())
    }

    /// mul — num1 num2 → num1*num2
    pub fn op_mul(&mut self) -> Result<(), String> {
        let (a, b, is_float) = self.pop_two_numeric()?;
        self.push_numeric(a * b, is_float);
        Ok(())
    }

    /// div — num1 num2 → num1/num2  (always float)
    pub fn op_div(&mut self) -> Result<(), String> {
        let (a, b, _) = self.pop_two_numeric()?;
        if b == 0.0 { return Err("div: division by zero".to_string()); }
        self.push(Value::Float(a / b));
        Ok(())
    }

    /// idiv — int1 int2 → int1/int2  (integer truncated toward zero)
    pub fn op_idiv(&mut self) -> Result<(), String> {
        let b = match self.pop()? {
            Value::Int(n) => n,
            other => return Err(format!("idiv: expected int, got {:?}", other)),
        };
        let a = match self.pop()? {
            Value::Int(n) => n,
            other => return Err(format!("idiv: expected int, got {:?}", other)),
        };
        if b == 0 { return Err("idiv: division by zero".to_string()); }
        self.push(Value::Int(a / b));
        Ok(())
    }

    /// mod — int1 int2 → int1 mod int2
    pub fn op_mod(&mut self) -> Result<(), String> {
        let b = match self.pop()? {
            Value::Int(n) => n,
            other => return Err(format!("mod: expected int, got {:?}", other)),
        };
        let a = match self.pop()? {
            Value::Int(n) => n,
            other => return Err(format!("mod: expected int, got {:?}", other)),
        };
        if b == 0 { return Err("mod: division by zero".to_string()); }
        self.push(Value::Int(a % b));
        Ok(())
    }

    // ── Unary operators ───────────────────────────────────────────────────────

    /// abs — num → |num|
    pub fn op_abs(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n.abs())),
            Value::Float(f) => self.push(Value::Float(f.abs())),
            other => return Err(format!("abs: expected number, got {:?}", other)),
        }
        Ok(())
    }

    /// neg — num → -num
    pub fn op_neg(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(-n)),
            Value::Float(f) => self.push(Value::Float(-f)),
            other => return Err(format!("neg: expected number, got {:?}", other)),
        }
        Ok(())
    }

    /// ceiling — num → smallest integer >= num
    pub fn op_ceiling(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n)),
            Value::Float(f) => self.push(Value::Float(f.ceil())),
            other => return Err(format!("ceiling: expected number, got {:?}", other)),
        }
        Ok(())
    }

    /// floor — num → greatest integer <= num
    pub fn op_floor(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n)),
            Value::Float(f) => self.push(Value::Float(f.floor())),
            other => return Err(format!("floor: expected number, got {:?}", other)),
        }
        Ok(())
    }

    /// round — num → nearest integer (0.5 rounds up)
    pub fn op_round(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n)),
            Value::Float(f) => self.push(Value::Float(f.round())),
            other => return Err(format!("round: expected number, got {:?}", other)),
        }
        Ok(())
    }

    /// sqrt — num → sqrt(num)  (always float)
    pub fn op_sqrt(&mut self) -> Result<(), String> {
        let n = self.pop_numeric()?;
        if n < 0.0 { return Err("sqrt: cannot take sqrt of negative number".to_string()); }
        self.push(Value::Float(n.sqrt()));
        Ok(())
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
    fn test_add_ints() {
        let mut s = stack_with(vec![Value::Int(3), Value::Int(4)]);
        s.op_add().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(7));
    }

    #[test]
    fn test_add_mixed() {
        let mut s = stack_with(vec![Value::Int(3), Value::Float(1.5)]);
        s.op_add().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(4.5));
    }

    #[test]
    fn test_sub() {
        let mut s = stack_with(vec![Value::Int(10), Value::Int(3)]);
        s.op_sub().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(7));
    }

    #[test]
    fn test_mul() {
        let mut s = stack_with(vec![Value::Int(4), Value::Int(5)]);
        s.op_mul().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(20));
    }

    #[test]
    fn test_div_always_float() {
        let mut s = stack_with(vec![Value::Int(7), Value::Int(2)]);
        s.op_div().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(3.5));
    }

    #[test]
    fn test_idiv() {
        let mut s = stack_with(vec![Value::Int(7), Value::Int(2)]);
        s.op_idiv().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(3));
    }

    #[test]
    fn test_mod() {
        let mut s = stack_with(vec![Value::Int(7), Value::Int(3)]);
        s.op_mod().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(1));
    }

    #[test]
    fn test_abs() {
        let mut s = stack_with(vec![Value::Int(-5)]);
        s.op_abs().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(5));
    }

    #[test]
    fn test_neg() {
        let mut s = stack_with(vec![Value::Int(3)]);
        s.op_neg().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(-3));
    }

    #[test]
    fn test_ceiling() {
        let mut s = stack_with(vec![Value::Float(3.2)]);
        s.op_ceiling().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(4.0));
    }

    #[test]
    fn test_floor() {
        let mut s = stack_with(vec![Value::Float(-4.8)]);
        s.op_floor().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(-5.0));
    }

    #[test]
    fn test_round() {
        let mut s = stack_with(vec![Value::Float(3.7)]);
        s.op_round().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(4.0));
    }

    #[test]
    fn test_sqrt() {
        let mut s = stack_with(vec![Value::Int(9)]);
        s.op_sqrt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(3.0));
    }

    #[test]
    fn test_div_by_zero() {
        let mut s = stack_with(vec![Value::Int(1), Value::Int(0)]);
        assert!(s.op_div().is_err());
    }
}