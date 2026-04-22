// strings.rs — String and array operators
//
// Implements PostScript string and array operations as methods on OperandStack.
//
// Operators:
//   length (string only)       — number of characters in a string
//   get                        — character code at index (strings) or element (arrays)
//   getinterval                — substring or sub-array
//   putinterval                — replace a range in a string in place
//   string                     — allocate a zero-filled string of length n
//   array                      — allocate an array of n zero values
//
// PostScript string notes:
//   - Strings are zero-indexed
//   - `get` on a string returns the integer character code (ASCII value)
//   - `getinterval` returns a new substring or sub-array
//   - `putinterval` mutates a string in place by replacing a range
//
// Note: `length` for dicts/arrays is routed via dictionary.rs.
// `get` for dicts is routed via dictionary.rs.
// The interpreter dispatch checks the top-of-stack type to route correctly.

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {
    /// length — string → int
    /// Push the number of characters in the string.
    ///   Before: (hello)    After: 5
    pub fn op_string_length(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Str(s) => {
                self.push(Value::Int(s.len() as i64));
                Ok(())
            }
            other => Err(format!("length: expected string, got {:?}", other)),
        }
    }

    /// get — string/array int → value
    /// For strings: push the ASCII code of the character at the given index.
    /// For arrays:  push the element at the given index.
    /// (Dict get is handled in dictionary.rs and routed from the interpreter.)
    ///   (hello) 1 get  →  101   (ASCII 'e')
    ///   [10 20] 0 get  →  10
    pub fn op_get(&mut self) -> Result<(), String> {
        let index = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => {
                return Err(format!(
                    "get: expected non-negative int index, got {:?}",
                    other
                ));
            }
        };
        match self.pop()? {
            Value::Str(s) => {
                let bytes = s.as_bytes();
                if index >= bytes.len() {
                    return Err(format!(
                        "get: index {} out of bounds for string of length {}",
                        index,
                        bytes.len()
                    ));
                }
                // PostScript get on a string returns the integer character code
                self.push(Value::Int(bytes[index] as i64));
                Ok(())
            }
            Value::Array(a) => {
                if index >= a.len() {
                    return Err(format!(
                        "get: index {} out of bounds for array of length {}",
                        index,
                        a.len()
                    ));
                }
                self.push(a[index].clone());
                Ok(())
            }
            other => Err(format!("get: expected string or array, got {:?}", other)),
        }
    }

    /// getinterval — string/array index count → string/array
    /// Push a new substring (or sub-array) starting at `index` with length `count`.
    ///   (hello) 1 3 getinterval  →  (ell)
    ///   [1 2 3] 0 2 getinterval  →  [1 2]
    pub fn op_getinterval(&mut self) -> Result<(), String> {
        let count = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => {
                return Err(format!(
                    "getinterval: expected non-negative count, got {:?}",
                    other
                ));
            }
        };
        let index = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => {
                return Err(format!(
                    "getinterval: expected non-negative index, got {:?}",
                    other
                ));
            }
        };
        match self.pop()? {
            Value::Str(s) => {
                if index + count > s.len() {
                    return Err(format!(
                        "getinterval: range {}..{} out of bounds for string of length {}",
                        index,
                        index + count,
                        s.len()
                    ));
                }
                self.push(Value::Str(s[index..index + count].to_string()));
                Ok(())
            }
            Value::Array(a) => {
                if index + count > a.len() {
                    return Err(format!(
                        "getinterval: range {}..{} out of bounds for array of length {}",
                        index,
                        index + count,
                        a.len()
                    ));
                }
                self.push(Value::Array(a[index..index + count].to_vec()));
                Ok(())
            }
            other => Err(format!(
                "getinterval: expected string or array, got {:?}",
                other
            )),
        }
    }

    /// putinterval — string index replacement → (modified string)
    /// Replace the substring starting at `index` with `replacement` in place.
    ///   (hello) 1 (XY) putinterval  →  (hXYlo)
    pub fn op_putinterval(&mut self) -> Result<(), String> {
        let replacement = match self.pop()? {
            Value::Str(s) => s,
            other => {
                return Err(format!(
                    "putinterval: expected replacement string, got {:?}",
                    other
                ));
            }
        };
        let index = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => {
                return Err(format!(
                    "putinterval: expected non-negative index, got {:?}",
                    other
                ));
            }
        };
        match self.pop()? {
            Value::Str(mut s) => {
                if index + replacement.len() > s.len() {
                    return Err(format!(
                        "putinterval: replacement at {} of length {} exceeds string length {}",
                        index,
                        replacement.len(),
                        s.len()
                    ));
                }
                // Replace the bytes in place
                let bytes = unsafe { s.as_bytes_mut() };
                for (i, b) in replacement.bytes().enumerate() {
                    bytes[index + i] = b;
                }
                self.push(Value::Str(s));
                Ok(())
            }
            other => Err(format!("putinterval: expected string, got {:?}", other)),
        }
    }

    /// string — int → string
    /// Allocate a zero-filled (null byte) string of length n.
    /// This matches the PostScript `string` operator which produces a mutable
    /// string buffer that can later be written with putinterval.
    ///   5 string  →  (\x00\x00\x00\x00\x00)
    pub fn op_string(&mut self) -> Result<(), String> {
        let n = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => {
                return Err(format!(
                    "string: expected non-negative int, got {:?}",
                    other
                ));
            }
        };
        self.push(Value::Str("\0".repeat(n)));
        Ok(())
    }

    /// array — int → array
    /// Allocate an array of n elements all initialised to integer zero.
    /// Elements can be set individually using `put`.
    ///   3 array  →  [0 0 0]
    pub fn op_array(&mut self) -> Result<(), String> {
        let n = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("array: expected non-negative int, got {:?}", other)),
        };
        self.push(Value::Array(vec![Value::Int(0); n]));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack_with(vals: Vec<Value>) -> OperandStack {
        let mut s = OperandStack::new();
        for v in vals {
            s.push(v);
        }
        s
    }

    #[test]
    fn test_string_length() {
        let mut s = stack_with(vec![Value::Str("hello".to_string())]);
        s.op_string_length().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(5));
    }

    #[test]
    fn test_get_char_code() {
        let mut s = stack_with(vec![Value::Str("hello".to_string()), Value::Int(0)]);
        s.op_get().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(104)); // 'h'
    }

    #[test]
    fn test_get_array_element() {
        let mut s = stack_with(vec![
            Value::Array(vec![Value::Int(10), Value::Int(20), Value::Int(30)]),
            Value::Int(1),
        ]);
        s.op_get().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(20));
    }

    #[test]
    fn test_getinterval() {
        let mut s = stack_with(vec![
            Value::Str("hello".to_string()),
            Value::Int(1),
            Value::Int(3),
        ]);
        s.op_getinterval().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("ell".to_string()));
    }

    #[test]
    fn test_putinterval() {
        let mut s = stack_with(vec![
            Value::Str("hello".to_string()),
            Value::Int(1),
            Value::Str("XY".to_string()),
        ]);
        s.op_putinterval().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("hXYlo".to_string()));
    }

    #[test]
    fn test_string_op() {
        let mut s = stack_with(vec![Value::Int(5)]);
        s.op_string().unwrap();
        match s.pop().unwrap() {
            Value::Str(s) => assert_eq!(s.len(), 5),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn test_array_op() {
        let mut s = stack_with(vec![Value::Int(3)]);
        s.op_array().unwrap();
        assert_eq!(
            s.pop().unwrap(),
            Value::Array(vec![Value::Int(0), Value::Int(0), Value::Int(0)])
        );
    }
}
