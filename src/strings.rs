// strings.rs — String operators
//
// Implements PostScript string operations as methods on OperandStack.
//
// Operators: length, get, getinterval, putinterval
//
// PostScript string notes:
//   - Strings are zero-indexed
//   - `get` returns the integer character code (ASCII value) at an index
//   - `getinterval` returns a new substring
//   - `putinterval` mutates a string in place by replacing a range
//
// Note: `length` for dictionaries is handled in dictionary.rs.
// The interpreter dispatch checks the top-of-stack type to route correctly.

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {

    /// length — string → int
    /// Push the number of characters in the string.
    ///   Before: (hello)
    ///   After:  5
    pub fn op_string_length(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Str(s) => {
                self.push(Value::Int(s.len() as i64));
                Ok(())
            }
            other => Err(format!("length: expected string, got {:?}", other)),
        }
    }

    /// get — string int → int
    /// Push the ASCII code of the character at the given index.
    ///   Before: (hello) 1
    ///   After:  101       (ASCII for 'e')
    pub fn op_get(&mut self) -> Result<(), String> {
        let index = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("get: expected non-negative int index, got {:?}", other)),
        };
        match self.pop()? {
            Value::Str(s) => {
                let bytes = s.as_bytes();
                if index >= bytes.len() {
                    return Err(format!("get: index {} out of bounds for string of length {}", index, bytes.len()));
                }
                // PostScript get returns the integer character code
                self.push(Value::Int(bytes[index] as i64));
                Ok(())
            }
            other => Err(format!("get: expected string, got {:?}", other)),
        }
    }

    /// getinterval — string index count → string
    /// Push a new substring starting at `index` with length `count`.
    ///   Before: (hello) 1 3
    ///   After:  (ell)
    pub fn op_getinterval(&mut self) -> Result<(), String> {
        let count = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("getinterval: expected non-negative count, got {:?}", other)),
        };
        let index = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("getinterval: expected non-negative index, got {:?}", other)),
        };
        match self.pop()? {
            Value::Str(s) => {
                if index + count > s.len() {
                    return Err(format!(
                        "getinterval: range {}..{} out of bounds for string of length {}",
                        index, index + count, s.len()
                    ));
                }
                let substr = s[index..index + count].to_string();
                self.push(Value::Str(substr));
                Ok(())
            }
            other => Err(format!("getinterval: expected string, got {:?}", other)),
        }
    }

    /// putinterval — string index replacement → (modified string)
    /// Replace the substring starting at `index` with `replacement` in place.
    ///   Before: (hello) 1 (XY)
    ///   After:  (hXYlo)
    pub fn op_putinterval(&mut self) -> Result<(), String> {
        let replacement = match self.pop()? {
            Value::Str(s) => s,
            other => return Err(format!("putinterval: expected replacement string, got {:?}", other)),
        };
        let index = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("putinterval: expected non-negative index, got {:?}", other)),
        };
        match self.pop()? {
            Value::Str(mut s) => {
                if index + replacement.len() > s.len() {
                    return Err(format!(
                        "putinterval: replacement at {} of length {} exceeds string length {}",
                        index, replacement.len(), s.len()
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
    fn test_string_length() {
        let mut s = stack_with(vec![Value::Str("hello".to_string())]);
        s.op_string_length().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(5));
    }

    #[test]
    fn test_string_length_empty() {
        let mut s = stack_with(vec![Value::Str("".to_string())]);
        s.op_string_length().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(0));
    }

    #[test]
    fn test_get_char_code() {
        // 'h' is ASCII 104
        let mut s = stack_with(vec![Value::Str("hello".to_string()), Value::Int(0)]);
        s.op_get().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(104));
    }

    #[test]
    fn test_get_middle() {
        // 'e' is ASCII 101
        let mut s = stack_with(vec![Value::Str("hello".to_string()), Value::Int(1)]);
        s.op_get().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Int(101));
    }

    #[test]
    fn test_get_out_of_bounds() {
        let mut s = stack_with(vec![Value::Str("hi".to_string()), Value::Int(5)]);
        assert!(s.op_get().is_err());
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
    fn test_getinterval_from_start() {
        let mut s = stack_with(vec![
            Value::Str("hello".to_string()),
            Value::Int(0),
            Value::Int(2),
        ]);
        s.op_getinterval().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("he".to_string()));
    }

    #[test]
    fn test_getinterval_out_of_bounds() {
        let mut s = stack_with(vec![
            Value::Str("hello".to_string()),
            Value::Int(3),
            Value::Int(5),
        ]);
        assert!(s.op_getinterval().is_err());
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
    fn test_putinterval_at_start() {
        let mut s = stack_with(vec![
            Value::Str("hello".to_string()),
            Value::Int(0),
            Value::Str("HE".to_string()),
        ]);
        s.op_putinterval().unwrap();
        assert_eq!(s.pop().unwrap(), Value::Str("HEllo".to_string()));
    }

    #[test]
    fn test_putinterval_out_of_bounds() {
        let mut s = stack_with(vec![
            Value::Str("hi".to_string()),
            Value::Int(1),
            Value::Str("XYZ".to_string()),
        ]);
        assert!(s.op_putinterval().is_err());
    }
}