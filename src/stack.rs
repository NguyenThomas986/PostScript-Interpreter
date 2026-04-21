// stack.rs — Operand stack and stack manipulation operators
//
// Owns the operand stack (Vec<Value>) and implements:
//   exch, pop, dup, copy, clear, count
//
// All other modules that need to push/pop go through the methods here.

use crate::types::Value;

pub struct OperandStack {
    data: Vec<Value>,
}

impl OperandStack {
    pub fn new() -> Self {
        OperandStack { data: Vec::new() }
    }

    // ── Core push / pop ───────────────────────────────────────────────────────

    /// Push a value onto the top of the stack
    pub fn push(&mut self, val: Value) {
        self.data.push(val);
    }

    /// Pop the top value, error if empty
    pub fn pop(&mut self) -> Result<Value, String> {
        self.data.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    /// Peek at the top value without removing it
    pub fn peek(&self) -> Result<&Value, String> {
        self.data.last().ok_or_else(|| "Stack underflow".to_string())
    }

    /// Current number of elements
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Read-only view of the underlying data (used by copy and display)
    pub fn as_slice(&self) -> &[Value] {
        &self.data
    }

    /// Extend the stack with a slice of values (used by copy)
    pub fn extend(&mut self, vals: Vec<Value>) {
        self.data.extend(vals);
    }

    /// Empty the entire stack
    pub fn clear(&mut self) {
        self.data.clear();
    }

    // ── Stack manipulation operators ──────────────────────────────────────────

    /// exch — swap the top two elements
    ///   Before: a b    After: b a
    pub fn op_exch(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(b);
        self.push(a);
        Ok(())
    }

    /// pop — discard top element
    pub fn op_pop(&mut self) -> Result<(), String> {
        self.pop()?;
        Ok(())
    }

    /// dup — duplicate top element
    ///   Before: a    After: a a
    pub fn op_dup(&mut self) -> Result<(), String> {
        let top = self.peek()?.clone();
        self.push(top);
        Ok(())
    }

    /// copy — duplicate top n elements in order
    ///   Before: a b c 2    After: a b c b c
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

    /// clear — remove all elements
    pub fn op_clear(&mut self) -> Result<(), String> {
        self.data.clear();
        Ok(())
    }

    /// count — push the number of elements currently on the stack
    ///   Before: a b c    After: a b c 3
    pub fn op_count(&mut self) -> Result<(), String> {
        let n = self.data.len() as i64;
        self.push(Value::Int(n));
        Ok(())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

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
        assert_eq!(s.pop().unwrap(), Value::Int(5));
        assert_eq!(s.pop().unwrap(), Value::Int(5));
    }

    #[test]
    fn test_copy() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        s.push(Value::Int(3));
        s.push(Value::Int(2)); // copy top 2
        s.op_copy().unwrap();
        assert_eq!(s.len(), 5);
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
}