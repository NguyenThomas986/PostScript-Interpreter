// stack.rs — Operand stack and stack manipulation operators
//
// Owns the operand stack (Vec<Value>) and implements:
//   exch, pop, dup, copy, clear, count  — original operators
//   roll, index                         — additional stack reordering
//   mark, cleartomark, counttomark      — mark-based operators
//
// All other modules that need to push/pop go through the methods here.

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

    /// roll — n j roll — rotate the top n elements j positions toward the top.
    ///   Positive j rotates upward; negative rotates downward.
    ///   Before: a b c  3 1 roll    After: c a b
    ///   Before: a b c  3 -1 roll   After: b c a
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
        // Normalize j into [0, n) so rotate_right never panics
        let j = ((j % n as i64) + n as i64) as usize % n;
        slice.rotate_right(j);
        Ok(())
    }

    /// index — push a copy of the element n positions from the top (0 = top).
    ///   Before: a b c  1 index    After: a b c b
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

    // ── Mark operators ────────────────────────────────────────────────────────

    /// mark — push a mark object onto the stack.
    /// Used as a sentinel to delimit a group of values for cleartomark /
    /// counttomark. The `[ ... ]` array literal also uses a mark internally.
    pub fn op_mark(&mut self) -> Result<(), String> {
        self.push(Value::Mark);
        Ok(())
    }

    /// cleartomark — pop everything down to and including the topmost mark.
    ///   Before: a -mark- b c    After: a
    pub fn op_cleartomark(&mut self) -> Result<(), String> {
        loop {
            if self.pop()? == Value::Mark {
                return Ok(());
            }
        }
    }

    /// counttomark — count elements above the topmost mark, push that count.
    /// Does NOT consume the mark or the elements.
    ///   Before: -mark- a b c    After: -mark- a b c 3
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
        // 1 2 3  3 1 roll → 3 1 2
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
    fn test_index() {
        // 1 2 3  1 index → 1 2 3 2
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.push(Value::Int(2));
        s.push(Value::Int(3));
        s.push(Value::Int(1));
        s.op_index().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(2));
    }

    #[test]
    fn test_mark_cleartomark() {
        let mut s = OperandStack::new();
        s.push(Value::Int(1));
        s.op_mark().unwrap();
        s.push(Value::Int(2));
        s.push(Value::Int(3));
        s.op_cleartomark().unwrap();
        // Only Value::Int(1) should remain
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
    fn test_is_empty() {
        let mut s = OperandStack::new();
        assert!(s.is_empty());
        s.push(Value::Int(1));
        assert!(!s.is_empty());
    }
}
