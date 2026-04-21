// dictionary.rs — Dictionary stack and dictionary operators
//
// Owns the dictionary stack (Vec<HashMap<String, Value>>) and implements:
//   dict, length, maxlength, begin, end, def
//
// Also exposes lookup() which walks the stack top-to-bottom to resolve names.
// This is the foundation of both dynamic and lexical scoping (Step 10).

use crate::types::Value;
use std::collections::HashMap;

// ── Dictionary value ──────────────────────────────────────────────────────────

/// A PostScript dictionary — a key/value store with a fixed capacity.
/// We track `capacity` separately because PostScript's `maxlength` operator
/// returns the capacity the dict was created with, not how many entries it has.
#[derive(Debug, Clone, PartialEq)]
pub struct Dict {
    pub capacity: usize,
    pub entries: HashMap<String, Value>,
}

impl Dict {
    pub fn new(capacity: usize) -> Self {
        Dict {
            capacity,
            entries: HashMap::new(),
        }
    }
}

// ── Dictionary stack ──────────────────────────────────────────────────────────

pub struct DictStack {
    /// The stack of dictionaries. The LAST element is the current (innermost) scope.
    stack: Vec<Dict>,
}

impl DictStack {
    pub fn new() -> Self {
        // Start with one global dictionary on the stack so `def` always has
        // somewhere to write even before the user calls `begin`.
        let mut ds = DictStack { stack: Vec::new() };
        ds.stack.push(Dict::new(256)); // global dict
        ds
    }

    // ── Core lookup ───────────────────────────────────────────────────────────

    /// Look up a name by walking the dictionary stack from top (innermost)
    /// to bottom (outermost). Returns the first match found, or None.
    ///
    /// This implements dynamic scoping — the call-time stack is searched.
    /// Lexical scoping (Step 10) will snapshot this stack at definition time.
    pub fn lookup(&self, name: &str) -> Option<Value> {
        // Walk from the top of the stack downward
        for dict in self.stack.iter().rev() {
            if let Some(val) = dict.entries.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    /// Define a name in the topmost (current) dictionary.
    /// This is what `def` does.
    pub fn define(&mut self, name: String, value: Value) -> Result<(), String> {
        match self.stack.last_mut() {
            Some(dict) => { dict.entries.insert(name, value); Ok(()) }
            None => Err("def: dictionary stack is empty".to_string()),
        }
    }

    /// Push a dictionary onto the stack (the `begin` operator)
    pub fn begin(&mut self, dict: Dict) {
        self.stack.push(dict);
    }

    /// Pop the topmost dictionary off the stack (the `end` operator)
    pub fn end(&mut self) -> Result<(), String> {
        // Never pop the last (global) dictionary
        if self.stack.len() <= 1 {
            return Err("end: cannot pop the global dictionary".to_string());
        }
        self.stack.pop();
        Ok(())
    }

    /// Return a snapshot of the entire stack — used for lexical scoping (Step 10)
    pub fn snapshot(&self) -> Vec<Dict> {
        self.stack.clone()
    }

    /// Swap the entire dict stack with a new one, returning the old one.
    /// Used by execute_procedure to temporarily install a captured environment.
    pub fn swap(&mut self, new_stack: Vec<Dict>) -> Vec<Dict> {
        std::mem::replace(&mut self.stack, new_stack)
    }

    /// Return a read-only reference to the stack (used by tests)
    pub fn as_slice(&self) -> &[Dict] {
        &self.stack
    }
}

// ── Dictionary operators (called from interpreter dispatch) ───────────────────

use crate::stack::OperandStack;

impl DictStack {
    /// dict — pop integer capacity, push a new empty dictionary onto operand stack
    ///   Before: n
    ///   After:  dict  (a new dictionary with capacity n)
    pub fn op_dict(&self, stack: &mut OperandStack) -> Result<(), String> {
        let n = match stack.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("dict: expected non-negative int, got {:?}", other)),
        };
        // We represent a dict on the operand stack as a special Value.
        // For now we push a placeholder — full dict-as-value support comes with begin/end.
        stack.push(Value::Dict(Dict::new(n)));
        Ok(())
    }

    /// length — pop a dict or string, push its current number of entries / characters
    ///   Before: dict   After: int
    pub fn op_length(&self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => stack.push(Value::Int(d.entries.len() as i64)),
            Value::Str(s)  => stack.push(Value::Int(s.len() as i64)),
            other => return Err(format!("length: expected dict or string, got {:?}", other)),
        }
        Ok(())
    }

    /// maxlength — pop a dict, push its capacity
    ///   Before: dict   After: int
    pub fn op_maxlength(&self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => stack.push(Value::Int(d.capacity as i64)),
            other => return Err(format!("maxlength: expected dict, got {:?}", other)),
        }
        Ok(())
    }

    /// begin — pop a dict from the operand stack and push it onto the dict stack
    ///   Before (operand): dict
    ///   After  (operand): (empty)
    ///   After  (dict stack): dict is now the current scope
    pub fn op_begin(&mut self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => { self.begin(d); Ok(()) }
            other => Err(format!("begin: expected dict, got {:?}", other)),
        }
    }

    /// end — pop the current dictionary off the dict stack
    pub fn op_end(&mut self) -> Result<(), String> {
        self.end()
    }

    /// def — pop value and name, bind name → value in the current dictionary
    ///   Before: /name value
    ///   After:  (empty)
    pub fn op_def(&mut self, stack: &mut OperandStack) -> Result<(), String> {
        let value = stack.pop()?;
        let name = match stack.pop()? {
            Value::Name(n) => n,
            other => return Err(format!("def: expected name, got {:?}", other)),
        };
        self.define(name, value)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_and_lookup() {
        let mut ds = DictStack::new();
        ds.define("x".to_string(), Value::Int(42)).unwrap();
        assert_eq!(ds.lookup("x"), Some(Value::Int(42)));
    }

    #[test]
    fn test_lookup_missing() {
        let ds = DictStack::new();
        assert_eq!(ds.lookup("nothing"), None);
    }

    #[test]
    fn test_inner_scope_shadows_outer() {
        let mut ds = DictStack::new();
        // Define x = 1 in the global dict
        ds.define("x".to_string(), Value::Int(1)).unwrap();

        // Push a new inner dict and define x = 99 there
        ds.begin(Dict::new(10));
        ds.define("x".to_string(), Value::Int(99)).unwrap();

        // Lookup should find the inner x first
        assert_eq!(ds.lookup("x"), Some(Value::Int(99)));

        // After end(), we should see the outer x again
        ds.end().unwrap();
        assert_eq!(ds.lookup("x"), Some(Value::Int(1)));
    }

    #[test]
    fn test_end_cannot_pop_global() {
        let mut ds = DictStack::new();
        assert!(ds.end().is_err());
    }

    #[test]
    fn test_op_def_and_lookup() {
        let mut ds = DictStack::new();
        let mut stack = OperandStack::new();

        // Push /x 42 then call def
        stack.push(Value::Name("x".to_string()));
        stack.push(Value::Int(42));
        ds.op_def(&mut stack).unwrap();

        assert_eq!(ds.lookup("x"), Some(Value::Int(42)));
        assert_eq!(stack.len(), 0); // def consumed both values
    }

    #[test]
    fn test_op_dict_begin_end() {
        let mut ds = DictStack::new();
        let mut stack = OperandStack::new();

        // Create a dict and push it onto the dict stack
        stack.push(Value::Int(10));
        ds.op_dict(&mut stack).unwrap();  // stack now has Dict(10)
        ds.op_begin(&mut stack).unwrap(); // dict stack grows

        assert_eq!(ds.as_slice().len(), 2); // global + new dict

        ds.op_end().unwrap();
        assert_eq!(ds.as_slice().len(), 1); // back to global only
    }
}