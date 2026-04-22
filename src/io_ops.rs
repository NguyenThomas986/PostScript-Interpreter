// io_ops.rs — Input/Output operators

use crate::stack::OperandStack;
use crate::types::Value;

impl OperandStack {
    pub fn op_print(&mut self) -> Result<(), String> {
        match self.pop()? {
            Value::Str(s) => {
                print!("{}", s);
                Ok(())
            }
            other => Err(format!("print: expected string, got {:?}", other)),
        }
    }

    pub fn op_print_pop(&mut self) -> Result<(), String> {
        let val = self.pop()?;
        println!("{}", val);
        Ok(())
    }

    pub fn op_print_repr(&mut self) -> Result<(), String> {
        let val = self.pop()?;
        println!("{}", postscript_repr(&val));
        Ok(())
    }
}

fn postscript_repr(val: &Value) -> String {
    match val {
        Value::Str(s) => format!("({})", s),
        Value::Name(n) => format!("/{}", n),
        Value::Procedure { tokens, .. } => format!("{{ {:?} }}", tokens),
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(postscript_repr).collect();
            format!("[{}]", inner.join(" "))
        }
        other => format!("{}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_postscript_repr_array() {
        let val = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(postscript_repr(&val), "[1 2]");
    }
}
