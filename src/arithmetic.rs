// arithmetic.rs — Arithmetic operators

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {
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

    fn pop_numeric(&mut self) -> Result<f64, String> {
        match self.pop()? {
            Value::Int(n)   => Ok(n as f64),
            Value::Float(f) => Ok(f),
            other => Err(format!("Expected number, got {:?}", other)),
        }
    }

    fn push_numeric(&mut self, value: f64, is_float: bool) {
        if is_float { self.push(Value::Float(value)); }
        else { self.push(Value::Int(value as i64)); }
    }

    pub fn op_add(&mut self) -> Result<(), String> {
        let (a, b, is_float) = self.pop_two_numeric()?;
        self.push_numeric(a + b, is_float); Ok(())
    }

    pub fn op_sub(&mut self) -> Result<(), String> {
        let (a, b, is_float) = self.pop_two_numeric()?;
        self.push_numeric(a - b, is_float); Ok(())
    }

    pub fn op_mul(&mut self) -> Result<(), String> {
        let (a, b, is_float) = self.pop_two_numeric()?;
        self.push_numeric(a * b, is_float); Ok(())
    }

    pub fn op_div(&mut self) -> Result<(), String> {
        let (a, b, _) = self.pop_two_numeric()?;
        if b == 0.0 { return Err("div: division by zero".to_string()); }
        self.push(Value::Float(a / b)); Ok(())
    }

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
        self.push(Value::Int(a / b)); Ok(())
    }

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
        self.push(Value::Int(a % b)); Ok(())
    }

    pub fn op_abs(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n.abs())),
            Value::Float(f) => self.push(Value::Float(f.abs())),
            other => return Err(format!("abs: expected number, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_neg(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(-n)),
            Value::Float(f) => self.push(Value::Float(-f)),
            other => return Err(format!("neg: expected number, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_ceiling(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n)),
            Value::Float(f) => self.push(Value::Float(f.ceil())),
            other => return Err(format!("ceiling: expected number, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_floor(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n)),
            Value::Float(f) => self.push(Value::Float(f.floor())),
            other => return Err(format!("floor: expected number, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_round(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Int(n)   => self.push(Value::Int(n)),
            Value::Float(f) => self.push(Value::Float(f.round())),
            other => return Err(format!("round: expected number, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_sqrt(&mut self) -> Result<(), String> {
        let n = self.pop_numeric()?;
        if n < 0.0 { return Err("sqrt: cannot take sqrt of negative number".to_string()); }
        self.push(Value::Float(n.sqrt())); Ok(())
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

    // ── add ──────────────────────────────────────────────────────────────────

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
    fn test_add_floats() {
        let mut s = stack_with(vec![Value::Float(1.5), Value::Float(2.5)]);
        s.op_add().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(4.0));
    }

    #[test]
    fn test_add_invalid_type() {
        let mut s = stack_with(vec![Value::Bool(true), Value::Int(1)]);
        assert!(s.op_add().is_err());
    }

    // ── sub ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_sub() {
        let mut s = stack_with(vec![Value::Int(10), Value::Int(3)]);
        s.op_sub().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(7));
    }

    #[test]
    fn test_sub_float() {
        let mut s = stack_with(vec![Value::Float(5.0), Value::Float(1.5)]);
        s.op_sub().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(3.5));
    }

    // ── mul ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_mul() {
        let mut s = stack_with(vec![Value::Int(4), Value::Int(5)]);
        s.op_mul().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(20));
    }

    #[test]
    fn test_mul_float() {
        let mut s = stack_with(vec![Value::Float(2.0), Value::Float(3.0)]);
        s.op_mul().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(6.0));
    }

    // ── div ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_div_always_float() {
        let mut s = stack_with(vec![Value::Int(7), Value::Int(2)]);
        s.op_div().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(3.5));
    }

    #[test]
    fn test_div_by_zero() {
        let mut s = stack_with(vec![Value::Int(1), Value::Int(0)]);
        assert!(s.op_div().is_err());
    }

    // ── idiv ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_idiv() {
        let mut s = stack_with(vec![Value::Int(7), Value::Int(2)]);
        s.op_idiv().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(3));
    }

    #[test]
    fn test_idiv_by_zero() {
        let mut s = stack_with(vec![Value::Int(5), Value::Int(0)]);
        assert!(s.op_idiv().is_err());
    }

    #[test]
    fn test_idiv_wrong_type_b() {
        let mut s = stack_with(vec![Value::Int(5), Value::Float(2.0)]);
        assert!(s.op_idiv().is_err());
    }

    #[test]
    fn test_idiv_wrong_type_a() {
        let mut s = stack_with(vec![Value::Float(5.0), Value::Int(2)]);
        assert!(s.op_idiv().is_err());
    }

    // ── mod ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_mod() {
        let mut s = stack_with(vec![Value::Int(7), Value::Int(3)]);
        s.op_mod().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(1));
    }

    #[test]
    fn test_mod_by_zero() {
        let mut s = stack_with(vec![Value::Int(5), Value::Int(0)]);
        assert!(s.op_mod().is_err());
    }

    #[test]
    fn test_mod_wrong_type_b() {
        let mut s = stack_with(vec![Value::Int(5), Value::Float(2.0)]);
        assert!(s.op_mod().is_err());
    }

    #[test]
    fn test_mod_wrong_type_a() {
        let mut s = stack_with(vec![Value::Float(5.0), Value::Int(2)]);
        assert!(s.op_mod().is_err());
    }

    // ── abs ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_abs_int() {
        let mut s = stack_with(vec![Value::Int(-5)]);
        s.op_abs().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(5));
    }

    #[test]
    fn test_abs_float() {
        let mut s = stack_with(vec![Value::Float(-3.5)]);
        s.op_abs().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(3.5));
    }

    #[test]
    fn test_abs_wrong_type() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        assert!(s.op_abs().is_err());
    }

    // ── neg ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_neg_int() {
        let mut s = stack_with(vec![Value::Int(3)]);
        s.op_neg().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(-3));
    }

    #[test]
    fn test_neg_float() {
        let mut s = stack_with(vec![Value::Float(2.5)]);
        s.op_neg().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(-2.5));
    }

    #[test]
    fn test_neg_wrong_type() {
        let mut s = stack_with(vec![Value::Bool(false)]);
        assert!(s.op_neg().is_err());
    }

    // ── ceiling / floor / round ───────────────────────────────────────────────

    #[test]
    fn test_ceiling_float() {
        let mut s = stack_with(vec![Value::Float(3.2)]);
        s.op_ceiling().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(4.0));
    }

    #[test]
    fn test_ceiling_int_passthrough() {
        let mut s = stack_with(vec![Value::Int(5)]);
        s.op_ceiling().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(5));
    }

    #[test]
    fn test_ceiling_wrong_type() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        assert!(s.op_ceiling().is_err());
    }

    #[test]
    fn test_floor_float() {
        let mut s = stack_with(vec![Value::Float(-4.8)]);
        s.op_floor().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(-5.0));
    }

    #[test]
    fn test_floor_int_passthrough() {
        let mut s = stack_with(vec![Value::Int(3)]);
        s.op_floor().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(3));
    }

    #[test]
    fn test_floor_wrong_type() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        assert!(s.op_floor().is_err());
    }

    #[test]
    fn test_round_float() {
        let mut s = stack_with(vec![Value::Float(3.7)]);
        s.op_round().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(4.0));
    }

    #[test]
    fn test_round_int_passthrough() {
        let mut s = stack_with(vec![Value::Int(7)]);
        s.op_round().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(7));
    }

    #[test]
    fn test_round_wrong_type() {
        let mut s = stack_with(vec![Value::Bool(true)]);
        assert!(s.op_round().is_err());
    }

    // ── sqrt ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_sqrt() {
        let mut s = stack_with(vec![Value::Int(9)]);
        s.op_sqrt().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Float(3.0));
    }

    #[test]
    fn test_sqrt_negative() {
        let mut s = stack_with(vec![Value::Int(-1)]);
        assert!(s.op_sqrt().is_err());
    }
}