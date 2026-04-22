// control.rs — Flow control operators

use crate::interpreter::Interpreter;
use crate::types::Value;

impl Interpreter {
    pub fn op_if(&mut self) -> Result<(), String> {
        let proc = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => return Err(format!("if: expected procedure, got {:?}", other)),
        };
        let condition = match self.stack.pop()? {
            Value::Bool(b) => b,
            other => return Err(format!("if: expected bool condition, got {:?}", other)),
        };
        if condition { self.execute_procedure(&proc, None)?; }
        Ok(())
    }

    pub fn op_ifelse(&mut self) -> Result<(), String> {
        let proc_false = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => return Err(format!("ifelse: expected procedure for false branch, got {:?}", other)),
        };
        let proc_true = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => return Err(format!("ifelse: expected procedure for true branch, got {:?}", other)),
        };
        let condition = match self.stack.pop()? {
            Value::Bool(b) => b,
            other => return Err(format!("ifelse: expected bool condition, got {:?}", other)),
        };
        if condition { self.execute_procedure(&proc_true, None)?; }
        else         { self.execute_procedure(&proc_false, None)?; }
        Ok(())
    }

    pub fn op_for(&mut self) -> Result<(), String> {
        let proc = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => return Err(format!("for: expected procedure, got {:?}", other)),
        };
        let limit = match self.stack.pop()? {
            Value::Int(n)   => n as f64,
            Value::Float(f) => f,
            other => return Err(format!("for: expected numeric limit, got {:?}", other)),
        };
        let increment = match self.stack.pop()? {
            Value::Int(n)   => n as f64,
            Value::Float(f) => f,
            other => return Err(format!("for: expected numeric increment, got {:?}", other)),
        };
        let init = match self.stack.pop()? {
            Value::Int(n)   => n as f64,
            Value::Float(f) => f,
            other => return Err(format!("for: expected numeric init, got {:?}", other)),
        };

        let use_float = init.fract() != 0.0 || increment.fract() != 0.0 || limit.fract() != 0.0;

        if increment == 0.0 { return Err("for: increment cannot be zero".to_string()); }

        let mut counter = init;
        loop {
            if increment > 0.0 && counter > limit { break; }
            if increment < 0.0 && counter < limit { break; }
            if use_float { self.stack.push(Value::Float(counter)); }
            else         { self.stack.push(Value::Int(counter as i64)); }
            self.execute_procedure(&proc, None)?;
            counter += increment;
        }
        Ok(())
    }

    pub fn op_repeat(&mut self) -> Result<(), String> {
        let proc = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => return Err(format!("repeat: expected procedure, got {:?}", other)),
        };
        let n = match self.stack.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("repeat: expected non-negative int, got {:?}", other)),
        };
        for _ in 0..n { self.execute_procedure(&proc, None)?; }
        Ok(())
    }

    pub fn op_quit(&mut self) -> Result<(), String> {
        Err("__quit__".to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::Value;

    fn run(source: &str) -> Vec<Value> {
        let mut interp = Interpreter::new();
        interp.run(source).expect("interpreter error");
        interp.stack.as_slice().to_vec()
    }

    #[test]
    fn test_if_true()  { assert_eq!(run("true { 42 } if"), vec![Value::Int(42)]); }
    #[test]
    fn test_if_false() { assert_eq!(run("false { 42 } if"), vec![]); }
    #[test]
    fn test_ifelse_true()  { assert_eq!(run("true { 1 } { 2 } ifelse"),  vec![Value::Int(1)]); }
    #[test]
    fn test_ifelse_false() { assert_eq!(run("false { 1 } { 2 } ifelse"), vec![Value::Int(2)]); }

    #[test]
    fn test_for_basic() {
        assert_eq!(run("1 1 3 { } for"),
            vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    }

    #[test]
    fn test_for_with_add() {
        assert_eq!(run("0 1 1 3 { add } for"), vec![Value::Int(6)]);
    }

    #[test]
    fn test_for_countdown() {
        assert_eq!(run("3 -1 1 { } for"),
            vec![Value::Int(3), Value::Int(2), Value::Int(1)]);
    }

    #[test]
    fn test_repeat() {
        assert_eq!(run("3 { 1 } repeat"),
            vec![Value::Int(1), Value::Int(1), Value::Int(1)]);
    }

    #[test]
    fn test_repeat_zero() {
        assert_eq!(run("0 { 99 } repeat"), vec![]);
    }

    #[test]
    fn test_quit_returns_sentinel() {
        let mut interp = Interpreter::new();
        let result = interp.run("quit");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "__quit__");
    }
}