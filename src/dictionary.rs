// dictionary.rs — Dictionary stack and dictionary operators
//
// Owns the dictionary stack (Vec<Dict>) and implements:
//   dict, length, maxlength, begin, end, def   — core dictionary operators
//   put                                         — store by key (dicts) or index (arrays)
//   get (dict/array variant)                    — retrieve by key or index
//   forall (scaffolding)                        — iteration; full execution delegated to interpreter.rs
//
// Also exposes lookup() which walks the stack top-to-bottom to resolve names.
// This is the foundation of both dynamic and lexical scoping.
//
// The dictionary stack is how PostScript implements scope. Every `begin` pushes
// a new Dict onto the stack; every `end` pops one off. Every `def` writes into
// the top dict. Name lookups walk the stack from top to bottom, returning the
// first match — so inner scopes shadow outer ones naturally.
//
// Under dynamic scoping this walk always uses the live stack at call time.
// Under lexical scoping, execute_procedure in interpreter.rs swaps the live
// stack out for a snapshot that was taken at definition time, so the lookup
// sees definition-time bindings even when called from a different scope later.

use crate::types::Value;
use std::collections::HashMap;

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

pub struct DictStack {
    stack: Vec<Dict>,
}

impl Default for DictStack {
    fn default() -> Self {
        Self::new()
    }
}

impl DictStack {
    pub fn new() -> Self {
        // We start with one global dictionary already on the stack. Any `def`
        // executed outside of an explicit `begin`/`end` block lands here.
        // This dict is never popped — `end` refuses to remove the last one.
        let mut ds = DictStack { stack: Vec::new() };
        ds.stack.push(Dict::new(256));
        ds
    }

    /// Walk the dictionary stack from top to bottom and return the first match.
    ///
    /// Iterating in reverse means the most recently pushed dictionary — the
    /// innermost scope — is checked first. This is what makes inner definitions
    /// shadow outer ones. Under dynamic scoping the walk always hits the live
    /// stack at the moment of the call. Under lexical scoping,
    /// execute_procedure has already swapped in a snapshot of the definition-time
    /// stack, so this same walk hits the bindings that were in effect when the
    /// procedure was defined, not when it is being called.
    pub fn lookup(&self, name: &str) -> Option<Value> {
        for dict in self.stack.iter().rev() {
            if let Some(val) = dict.entries.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    /// Write name → value into the top dictionary on the stack.
    ///
    /// This is what `def` calls. It always targets the top dict, which is why
    /// `begin` controls which scope a definition lands in: pushing a new dict
    /// with `begin` redirects subsequent `def` calls into that inner scope,
    /// leaving the outer scope untouched.
    pub fn define(&mut self, name: String, value: Value) -> Result<(), String> {
        match self.stack.last_mut() {
            Some(dict) => {
                dict.entries.insert(name, value);
                Ok(())
            }
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

    /// Clone the entire dictionary stack into a Vec<Dict>.
    ///
    /// This is how lexical scoping captures the definition-time environment.
    /// When `use_lexical_scoping` is true and the interpreter encounters a `{`,
    /// it calls snapshot() right then and stores the clone inside the resulting
    /// Value::Procedure as `captured_env`. Later, when the procedure is called,
    /// execute_procedure swaps this snapshot in so name lookups see the
    /// definition-time bindings. The clone is intentionally deep — we want an
    /// independent copy that will not be affected by any future mutations to the
    /// live stack.
    pub fn snapshot(&self) -> Vec<Dict> {
        self.stack.clone()
    }

    /// Atomically replace the entire dictionary stack and return the old one.
    ///
    /// This is the mechanism that makes lexical scoping work at call time.
    /// execute_procedure calls this once to swap in the captured snapshot before
    /// running the procedure body, and calls it again afterward to restore the
    /// live stack — even if the procedure returned an error. Using
    /// std::mem::replace means both the replacement and the retrieval of the old
    /// value happen in a single step, so neither stack is ever lost or leaked.
    pub fn swap(&mut self, new_stack: Vec<Dict>) -> Vec<Dict> {
        std::mem::replace(&mut self.stack, new_stack)
    }

    pub fn as_slice(&self) -> &[Dict] {
        &self.stack
    }
}

use crate::stack::OperandStack;

impl DictStack {
    /// dict — int → dict
    ///
    /// Pops a non-negative integer capacity hint and pushes a new empty
    /// dictionary onto the operand stack. The dictionary is not yet active —
    /// you must follow up with `begin` to push it onto the dictionary stack and
    /// make it the current scope. The capacity is stored on the Dict but is only
    /// a hint; the underlying HashMap grows dynamically regardless of this value.
    pub fn op_dict(&self, stack: &mut OperandStack) -> Result<(), String> {
        let n = match stack.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("dict: expected non-negative int, got {:?}", other)),
        };
        stack.push(Value::Dict(Dict::new(n)));
        Ok(())
    }

    /// length — container → int
    ///
    /// Returns the number of entries currently in a dict, the number of elements
    /// in an array, or the number of bytes in a string. All three container types
    /// share this operator name. The value is computed from the actual live
    /// contents, not from any pre-declared capacity.
    pub fn op_length(&self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => stack.push(Value::Int(d.entries.len() as i64)),
            Value::Str(s) => stack.push(Value::Int(s.len() as i64)),
            Value::Array(a) => stack.push(Value::Int(a.len() as i64)),
            other => {
                return Err(format!(
                    "length: expected dict, string, or array, got {:?}",
                    other
                ));
            }
        }
        Ok(())
    }

    /// maxlength — dict → int
    ///
    /// Returns the declared capacity of a dictionary — the hint that was passed
    /// to `dict` when the dictionary was created. This is distinct from `length`,
    /// which returns how many entries are actually stored. In a production
    /// PostScript interpreter this value matters for memory management; here it
    /// is simply returned as stored.
    pub fn op_maxlength(&self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => stack.push(Value::Int(d.capacity as i64)),
            other => return Err(format!("maxlength: expected dict, got {:?}", other)),
        }
        Ok(())
    }

    /// begin — dict →
    ///
    /// Pops a dictionary from the operand stack and pushes it onto the
    /// dictionary stack, making it the new current scope. Any `def` that follows
    /// will write into this dictionary rather than into whatever was previously
    /// on top. The outer dictionaries are still reachable for name lookups —
    /// they are just searched after this inner one. To close the scope and
    /// return to the outer dict, call `end`.
    pub fn op_begin(&mut self, stack: &mut OperandStack) -> Result<(), String> {
        match stack.pop()? {
            Value::Dict(d) => {
                self.begin(d);
                Ok(())
            }
            other => Err(format!("begin: expected dict, got {:?}", other)),
        }
    }

    /// end →
    ///
    /// Pops the top dictionary off the dictionary stack, restoring the outer
    /// scope. Any names that were defined in the now-removed dict are no longer
    /// reachable — the outer dicts will be searched instead. Attempting to pop
    /// the last (global) dictionary is an error because the interpreter requires
    /// at least one dict on the stack at all times.
    pub fn op_end(&mut self) -> Result<(), String> {
        self.end()
    }

    /// def — /name value →
    ///
    /// Pops a value and then a name off the operand stack and binds the name to
    /// the value in the top dictionary. The name must be a LiteralName (i.e. it
    /// was written with a leading slash, like `/x`). The value is popped first
    /// because it sits on top; the name is popped second from just below it.
    /// No value is left on the operand stack — the only effect is the new entry
    /// in the current dictionary.
    pub fn op_def(&mut self, stack: &mut OperandStack) -> Result<(), String> {
        let value = stack.pop()?;
        let name = match stack.pop()? {
            Value::Name(n) => n,
            other => return Err(format!("def: expected name, got {:?}", other)),
        };
        self.define(name, value)
    }

    /// put — container key value →  (mutated container is left on the stack)
    ///
    /// Inserts or replaces an entry in a dict or an element in an array.
    ///   Dict form:  dict /key value put  → updated dict
    ///   Array form: array index value put → updated array
    ///
    /// Rust ownership note: PostScript expects dictionaries to behave as shared
    /// mutable references, so two variables pointing to the same dict would both
    /// see a `put`. Implementing that properly requires Rc<RefCell<>> throughout
    /// the value representation, which would substantially complicate the codebase.
    /// Instead, this implementation clones the container, mutates the clone, and
    /// pushes it back onto the stack. As a result, a dict that was stored via
    /// `def` will not see mutations applied to a separate copy of that dict on
    /// the operand stack. This limitation is documented in the README.
    pub fn op_put(&self, stack: &mut OperandStack) -> Result<(), String> {
        let value = stack.pop()?;
        let key = stack.pop()?;
        let container = stack.pop()?;
        match container {
            Value::Dict(mut d) => {
                let name = match key {
                    Value::Name(n) => n,
                    Value::Str(s) => s,
                    other => {
                        return Err(format!(
                            "put: dict key must be name or string, got {:?}",
                            other
                        ));
                    }
                };
                d.entries.insert(name, value);
                stack.push(Value::Dict(d));
                Ok(())
            }
            Value::Array(mut a) => {
                let idx = match key {
                    Value::Int(n) if n >= 0 => n as usize,
                    other => {
                        return Err(format!(
                            "put: array index must be non-negative int, got {:?}",
                            other
                        ));
                    }
                };
                if idx >= a.len() {
                    return Err(format!(
                        "put: index {} out of bounds for array of length {}",
                        idx,
                        a.len()
                    ));
                }
                a[idx] = value;
                stack.push(Value::Array(a));
                Ok(())
            }
            other => Err(format!("put: expected dict or array, got {:?}", other)),
        }
    }

    /// get — dict/array key → value
    ///
    /// Retrieves a single value from a dict or array by key or index. For dicts
    /// the key must be a Name or String; for arrays the key must be a
    /// non-negative integer index. Missing keys and out-of-bounds indices are
    /// both hard errors — there is no "not found" value in PostScript.
    ///
    /// String indexing and plain array indexing that does not involve a dict are
    /// handled in strings.rs and routed from the interpreter based on the
    /// container type sitting below the key on the stack, so this method only
    /// needs to cover dicts and arrays.
    pub fn op_get_dict(&self, stack: &mut OperandStack) -> Result<(), String> {
        let key = stack.pop()?;
        let container = stack.pop()?;
        match container {
            Value::Dict(d) => {
                let name = match key {
                    Value::Name(n) => n,
                    Value::Str(s) => s,
                    other => {
                        return Err(format!(
                            "get: dict key must be name or string, got {:?}",
                            other
                        ));
                    }
                };
                match d.entries.get(&name) {
                    Some(v) => {
                        stack.push(v.clone());
                        Ok(())
                    }
                    None => Err(format!("get: key '{}' not found in dict", name)),
                }
            }
            Value::Array(a) => {
                let idx = match key {
                    Value::Int(n) if n >= 0 => n as usize,
                    other => {
                        return Err(format!(
                            "get: array index must be non-negative int, got {:?}",
                            other
                        ));
                    }
                };
                if idx >= a.len() {
                    return Err(format!(
                        "get: index {} out of bounds for array of length {}",
                        idx,
                        a.len()
                    ));
                }
                stack.push(a[idx].clone());
                Ok(())
            }
            other => Err(format!("get: expected dict or array, got {:?}", other)),
        }
    }

    /// forall — array/dict/string proc →
    ///
    /// Scaffolding only. The full forall implementation lives in interpreter.rs
    /// because it needs to call execute_procedure, which requires access to the
    /// entire Interpreter struct. DictStack does not have that access.
    ///
    /// This method peeks at the container type, then re-pushes the container and
    /// procedure back onto the operand stack and returns a sentinel error string.
    /// The interpreter's dispatch loop catches the sentinel and calls the real
    /// op_forall on the Interpreter — which has execute_procedure available and
    /// can complete the iteration properly.
    pub fn op_forall(&mut self, stack: &mut OperandStack) -> Result<(), String> {
        let proc = match stack.pop()? {
            Value::Procedure {
                tokens,
                captured_env,
            } => (tokens, captured_env),
            other => return Err(format!("forall: expected procedure, got {:?}", other)),
        };
        let container = stack.pop()?;
        match container {
            Value::Array(a) => {
                stack.push(Value::Array(a));
                stack.push(Value::Procedure {
                    tokens: proc.0,
                    captured_env: proc.1,
                });
                Err("__forall_array__".to_string())
            }
            Value::Dict(d) => {
                stack.push(Value::Dict(d));
                stack.push(Value::Procedure {
                    tokens: proc.0,
                    captured_env: proc.1,
                });
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