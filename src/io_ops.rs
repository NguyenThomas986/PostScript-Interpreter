// io_ops.rs — Input/Output operators
//
// Implements: print, =, ==
//
// PostScript I/O rules:
//   - print  : pops a string and writes it to stdout with no newline
//   - =      : pops any value, prints it as plain text, adds a newline
//   - ==     : pops any value, prints it in PostScript syntax, adds a newline
//              (strings are wrapped in parens, names get a leading /)

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {

    /// print — pop a string and write it to stdout (no newline)
    ///   Before: (hello)
    ///   After:  (empty)   stdout: hello
    pub fn op_print(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Str(s) => {
                print!("{}", s);
                Ok(())
            }
            other => Err(format!("print: expected string, got {:?}", other)),
        }
    }

    /// = — pop any value, print it as plain text followed by a newline
    ///   Before: 42
    ///   After:  (empty)   stdout: 42\n
    pub fn op_print_pop(&mut self) -> Result<(), String> {
        let val = self.pop()?;
        println!("{}", val);
        Ok(())
    }

    /// == — pop any value, print it in PostScript representation followed by newline
    /// Differences from =:
    ///   - Strings are wrapped in parentheses:  (hello)
    ///   - Names get a leading slash:            /foo
    ///   - Everything else prints the same as =
    ///   Before: (hello)
    ///   After:  (empty)   stdout: (hello)\n
    pub fn op_print_repr(&mut self) -> Result<(), String> {
        let val = self.pop()?;
        println!("{}", postscript_repr(&val));
        Ok(())
    }
}

/// Format a Value in PostScript source representation.
/// Used by the == operator.
fn postscript_repr(val: &Value) -> String {
    match val {
        // Strings are shown with surrounding parens — PostScript string syntax
        Value::Str(s)  => format!("({})", s),
        // Names get their leading slash back
        Value::Name(n) => format!("/{}", n),
        // Procedures show as { token token ... }
        Value::Procedure { tokens, .. } => format!("{{ {:?} }}", tokens),
        // Everything else uses the standard Display impl
        other => format!("{}", other),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Note: we can't easily capture stdout in unit tests, so we test that
    // the operators consume the value correctly and don't error.

    #[test]
    fn test_print_consumes_string() {
        let mut s = OperandStack::new();
        s.push(Value::Str("hello".to_string()));
        s.op_print().unwrap();
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_print_rejects_non_string() {
        let mut s = OperandStack::new();
        s.push(Value::Int(42));
        assert!(s.op_print().is_err());
    }

    #[test]
    fn test_print_pop_consumes_value() {
        let mut s = OperandStack::new();
        s.push(Value::Int(42));
        s.op_print_pop().unwrap();
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_print_repr_consumes_value() {
        let mut s = OperandStack::new();
        s.push(Value::Str("hello".to_string()));
        s.op_print_repr().unwrap();
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_postscript_repr_string() {
        let val = Value::Str("hello".to_string());
        assert_eq!(postscript_repr(&val), "(hello)");
    }

    #[test]
    fn test_postscript_repr_name() {
        let val = Value::Name("foo".to_string());
        assert_eq!(postscript_repr(&val), "/foo");
    }

    #[test]
    fn test_postscript_repr_int() {
        let val = Value::Int(42);
        assert_eq!(postscript_repr(&val), "42");
    }

    #[test]
    fn test_postscript_repr_bool() {
        let val = Value::Bool(true);
        assert_eq!(postscript_repr(&val), "true");
    }
}