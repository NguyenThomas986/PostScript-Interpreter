// stack.rs — Operand stack and stack manipulation operators

use crate::types::Value;

pub struct OperandStack {
    data: Vec<Value>,
}

impl Default for OperandStack {
    fn default() -> Self {
        Self::new()
    }
}

impl OperandStack {
    pub fn new() -> Self {
        OperandStack { data: Vec::new() }
    }

    pub fn push(&mut self, val: Value) {
        self.data.push(val);
    }

    pub fn pop(&mut self) -> Result<Value, String> {
        self.data.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    pub fn peek(&self) -> Result<&Value, String> {
        self.data
            .last()
            .ok_or_else(|| "Stack underflow".to_string())
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[Value] {
        &self.data
    }

    pub fn extend(&mut self, vals: Vec<Value>) {
        self.data.extend(vals);
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn op_exch(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(b);
        self.push(a);
        Ok(())
    }

    pub fn op_pop(&mut self) -> Result<(), String> {
        self.pop()?;
        Ok(())
    }

    pub fn op_dup(&mut self) -> Result<(), String> {
        let top = self.peek()?.clone();
        self.push(top);
        Ok(())
    }

    pub fn op_copy(&mut self) -> Result<(), String> {
        let n = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("copy: expected non-negative int, got {:?}", other)),
        };
        let len = self.data.len();
        if n > len {
            return Err(format!("copy: requested {} but stack only has {}", n, len));
        }
        let top_n: Vec<Value> = self.data[len - n..].to_vec();
        self.data.extend(top_n);
        Ok(())
    }

    pub fn op_clear(&mut self) -> Result<(), String> {
        self.data.clear();
        Ok(())
    }

    pub fn op_count(&mut self) -> Result<(), String> {
        let n = self.data.len() as i64;
        self.push(Value::Int(n));
        Ok(())
    }

    pub fn op_roll(&mut self) -> Result<(), String> {
        let j = match self.pop()? {
            Value::Int(n) => n,
            other => return Err(format!("roll: expected int for j, got {:?}", other)),
        };
        let n = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => {
                return Err(format!(
                    "roll: expected non-negative int for n, got {:?}",
                    other
                ));
            }
        };
        if n == 0 {
            return Ok(());
        }
        let len = self.data.len();
        if n > len {
            return Err(format!("roll: n={} but stack only has {}", n, len));
        }
        let start = len - n;
        let slice = &mut self.data[start..];
        let j = ((j % n as i64) + n as i64) as usize % n;
        slice.rotate_right(j);
        Ok(())
    }

    pub fn op_index(&mut self) -> Result<(), String> {
        let n = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("index: expected non-negative int, got {:?}", other)),
        };
        let len = self.data.len();
        if n >= len {
            return Err(format!("index: {} out of bounds (stack size {})", n, len));
        }
        let val = self.data[len - 1 - n].clone();
        self.push(val);
        Ok(())
    }

    pub fn op_mark(&mut self) -> Result<(), String> {
        self.push(Value::Mark);
        Ok(())
    }

    pub fn op_cleartomark(&mut self) -> Result<(), String> {
        loop {
            if self.pop()? == Value::Mark {
                return Ok(());
            }
        }
    }

    pub fn op_counttomark(&mut self) -> Result<(), String> {
        let mut count = 0usize;
        for val in self.data.iter().rev() {
            if *val == Value::Mark {
                break;
            }
            count += 1;
        }
        if !self.data.contains(&Value::Mark) {
            return Err("counttomark: no mark on stack".to_string());
        }
        self.push(Value::Int(count as i64));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        assert_eq!(s.pop().unwrap(), Value::Int(2));
        assert_eq!(s.pop().unwrap(), Value::Int(1));
    }

    #[test]
    fn test_underflow() {
        let mut s = OperandStack::new();
        assert!(s.pop().is_err());
    }

    #[test]
    fn test_peek_empty() {
        let s = OperandStack::new();
        assert!(s.peek().is_err());
    }

    #[test]
    fn test_is_empty() {
        let mut s = OperandStack::new();
        assert!(s.is_empty());
        s.push(Value::Int(1));
        assert!(!s.is_empty());
    }

    #[test]
    fn test_exch() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        s.op_exch().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(1));
        assert_eq!(s.pop().unwrap(), Value::Int(2));
    }

    #[test]
    fn test_dup() {
        let mut s = OperandStack::new();
        s.push(Value::Int(5));
        s.op_dup().unwrap();
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_copy() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        s.push(Value::Int(3));
        s.push(Value::Int(2));
        s.op_copy().unwrap();
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn test_copy_overflow() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(5));
        assert!(s.op_copy().is_err());
    }

    #[test]
    fn test_copy_wrong_type() {
        let mut s = OperandStack::new();
        s.push(Value::Bool(true));
        assert!(s.op_copy().is_err());
    }

    #[test]
    fn test_clear() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        s.op_clear().unwrap();
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_count() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        s.op_count().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(2));
    }

    #[test]
    fn test_roll() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        s.push(Value::Int(3));
        s.push(Value::Int(3));
        s.push(Value::Int(1));
        s.op_roll().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(2));
        assert_eq!(s.pop().unwrap(), Value::Int(1));
        assert_eq!(s.pop().unwrap(), Value::Int(3));
    }

    #[test]
    fn test_roll_zero_n() {
        let mut s = OperandStack::new();
        s.push(Value::Int(0));
        s.push(Value::Int(0));
        s.op_roll().unwrap();
    }

    #[test]
    fn test_roll_n_exceeds_stack() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(99));
        s.push(Value::Int(1));
        assert!(s.op_roll().is_err());
    }

    #[test]
    fn test_roll_wrong_type_j() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Bool(true));
        assert!(s.op_roll().is_err());
    }

    #[test]
    fn test_roll_wrong_type_n() {
        let mut s = OperandStack::new();
        s.push(Value::Float(1.0));
        s.push(Value::Int(1));
        assert!(s.op_roll().is_err());
    }

    #[test]
    fn test_index() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        s.push(Value::Int(3));
        s.push(Value::Int(1));
        s.op_index().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(2));
    }

    #[test]
    fn test_index_out_of_bounds() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(5));
        assert!(s.op_index().is_err());
    }

    #[test]
    fn test_index_wrong_type() {
        let mut s = OperandStack::new();
        s.push(Value::Bool(true));
        assert!(s.op_index().is_err());
    }

    #[test]
    fn test_mark_cleartomark() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.op_mark().unwrap();
        s.push(Value::Int(2));
        s.push(Value::Int(3));
        s.op_cleartomark().unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s.pop().unwrap(), Value::Int(1));
    }

    #[test]
    fn test_counttomark() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.op_mark().unwrap();
        s.push(Value::Int(2));
        s.push(Value::Int(3));
        s.op_counttomark().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(2));
    }

    #[test]
    fn test_counttomark_no_mark() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        assert!(s.op_counttomark().is_err());
    }
}
