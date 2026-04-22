// dictionary.rs — Dictionary stack and dictionary operators
//
// Owns the dictionary stack (Vec<Dict>) and implements:
//   dict, length, maxlength, begin, end, def   — original operators
//   put                                         — store by key (dicts) or index (arrays)
//   get (dict/array variant)                    — retrieve by key or index
//   forall (scaffolding)                        — iteration; full execution in interpreter.rs
//
// Also exposes lookup() which walks the stack top-to-bottom to resolve names.
// This is the foundation of both dynamic and lexical scoping.

use crate::types::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Dict {
    pub capacity: usize,
    pub entries: HashMap<String, Value>,
}

impl Dict {
    pub fn new(capacity: usize) -> Self {
        Dict { capacity, entries: HashMap::new() }
    }
}

pub struct DictStack {
    stack: Vec<Dict>,
}

impl DictStack {
    pub fn new() -> Self {
        let mut ds = DictStack { stack: Vec::new() };
        ds.stack.push(Dict::new(256));
        ds
    }

    pub fn lookup(&self, name: &str) -> Option<Value> {
        for dict in self.stack.iter().rev() {
            if let Some(val) = dict.entries.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    pub fn define(&mut self, name: String, value: Value) -> Result<(), String> {
        match self.stack.last_mut() {
            Some(dict) => { dict.entries.insert(name, value); Ok(()) }
            None => Err("def: dictionary stack is empty".to_string()),
        }
    }

    pub fn begin(&mut self, dict: Dict) {
        self.stack.push(dict);
    }

    pub fn end(&mut self) -> Result<(), String> {
        if self.stack.len() <= 1 {
            return Err("end: cannot pop the global dictionary".to_string());
        }
        self.stack.pop();
        Ok(())
    }

    pub fn snapshot(&self) -> Vec<Dict> {
        self.stack.clone()
    }

    pub fn swap(&mut self, new_stack: Vec<Dict>) -> Vec<Dict> {
        std::mem::replace(&mut self.stack, new_stack)
    }

    pub fn as_slice(&self) -> &[Dict] {
        &self.stack
    }
}

use crate::stack::OperandStack;

impl DictStack {
    pub fn op_dict(&self, stack: &mut OperandStack) -> Result<(), String> {
        let n = match stack.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("dict: expected non-negative int, got {:?}", other)),
        };
        stack.push(Value::Dict(Dict::new(n)));
        Ok(())
    }

    pub fn op_length(&self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => stack.push(Value::Int(d.entries.len() as i64)),
            Value::Str(s)  => stack.push(Value::Int(s.len() as i64)),
            Value::Array(a) => stack.push(Value::Int(a.len() as i64)),
            other => return Err(format!("length: expected dict, string, or array, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_maxlength(&self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => stack.push(Value::Int(d.capacity as i64)),
            other => return Err(format!("maxlength: expected dict, got {:?}", other)),
        }
        Ok(())
    }

    pub fn op_begin(&mut self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => { self.begin(d); Ok(()) }
            other => Err(format!("begin: expected dict, got {:?}", other)),
        }
    }

    pub fn op_end(&mut self) -> Result<(), String> {
        self.end()
    }

    pub fn op_def(&mut self, stack: &mut OperandStack) -> Result<(), String> {
        let value = stack.pop()?;
        let name = match stack.pop()? {
            Value::Name(n) => n,
            other => return Err(format!("def: expected name, got {:?}", other)),
        };
        self.define(name, value)
    }

    /// put — container key value →  (mutates container and leaves it on the stack)
    /// For dicts:  dict /key value put
    /// For arrays: array index value put
    ///
    /// Note: this implementation leaves the modified container on the operand stack,
    /// which matches the common usage pattern of immediately continuing to work with it.
    pub fn op_put(&self, stack: &mut OperandStack) -> Result<(), String> {
        let value = stack.pop()?;
        let key   = stack.pop()?;
        let container = stack.pop()?;
        match container {
            Value::Dict(mut d) => {
                let name = match key {
                    Value::Name(n) => n,
                    Value::Str(s)  => s,
                    other => return Err(format!("put: dict key must be name or string, got {:?}", other)),
                };
                d.entries.insert(name, value);
                stack.push(Value::Dict(d));
                Ok(())
            }
            Value::Array(mut a) => {
                let idx = match key {
                    Value::Int(n) if n >= 0 => n as usize,
                    other => return Err(format!("put: array index must be non-negative int, got {:?}", other)),
                };
                if idx >= a.len() {
                    return Err(format!("put: index {} out of bounds for array of length {}", idx, a.len()));
                }
                a[idx] = value;
                stack.push(Value::Array(a));
                Ok(())
            }
            other => Err(format!("put: expected dict or array, got {:?}", other)),
        }
    }

    /// get — dict/array key → value
    /// For dicts:  look up a value by name or string key.
    /// For arrays: return the element at the given integer index.
    /// (String get and plain array get are handled in strings.rs.)
    pub fn op_get_dict(&self, stack: &mut OperandStack) -> Result<(), String> {
        let key = stack.pop()?;
        let container = stack.pop()?;
        match container {
            Value::Dict(d) => {
                let name = match key {
                    Value::Name(n) => n,
                    Value::Str(s)  => s,
                    other => return Err(format!("get: dict key must be name or string, got {:?}", other)),
                };
                match d.entries.get(&name) {
                    Some(v) => { stack.push(v.clone()); Ok(()) }
                    None => Err(format!("get: key '{}' not found in dict", name)),
                }
            }
            Value::Array(a) => {
                let idx = match key {
                    Value::Int(n) if n >= 0 => n as usize,
                    other => return Err(format!("get: array index must be non-negative int, got {:?}", other)),
                };
                if idx >= a.len() {
                    return Err(format!("get: index {} out of bounds for array of length {}", idx, a.len()));
                }
                stack.push(a[idx].clone());
                Ok(())
            }
            other => Err(format!("get: expected dict or array, got {:?}", other)),
        }
    }

    /// forall — array/dict/string proc →
    /// Iterate over a container and execute proc for each element.
    ///
    /// This method is a thin scaffold that re-pushes the container and
    /// procedure and signals the interpreter via a sentinel error so the
    /// full execution (which requires calling execute_procedure) can happen
    /// in interpreter.rs where the Interpreter struct is in scope.
    ///
    /// See Interpreter::op_forall in interpreter.rs for the real implementation.
    pub fn op_forall(&mut self, stack: &mut OperandStack) -> Result<(), String> {
        let proc = match stack.pop()? {
            Value::Procedure { tokens, captured_env } => (tokens, captured_env),
            other => return Err(format!("forall: expected procedure, got {:?}", other)),
        };
        let container = stack.pop()?;
        match container {
            Value::Array(a) => {
                stack.push(Value::Array(a));
                stack.push(Value::Procedure { tokens: proc.0, captured_env: proc.1 });
                Err("__forall_array__".to_string())
            }
            Value::Dict(d) => {
                stack.push(Value::Dict(d));
                stack.push(Value::Procedure { tokens: proc.0, captured_env: proc.1 });
                Err("__forall_dict__".to_string())
            }
            other => Err(format!("forall: expected array or dict, got {:?}", other)),
        }
    }
}

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
        ds.define("x".to_string(), Value::Int(1)).unwrap();
        ds.begin(Dict::new(10));
        ds.define("x".to_string(), Value::Int(99)).unwrap();
        assert_eq!(ds.lookup("x"), Some(Value::Int(99)));
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
        stack.push(Value::Name("x".to_string()));
        stack.push(Value::Int(42));
        ds.op_def(&mut stack).unwrap();
        assert_eq!(ds.lookup("x"), Some(Value::Int(42)));
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_op_dict_begin_end() {
        let mut ds = DictStack::new();
        let mut stack = OperandStack::new();
        stack.push(Value::Int(10));
        ds.op_dict(&mut stack).unwrap();
        ds.op_begin(&mut stack).unwrap();
        assert_eq!(ds.as_slice().len(), 2);
        ds.op_end().unwrap();
        assert_eq!(ds.as_slice().len(), 1);
    }
}