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
        if condition {
            self.execute_procedure(&proc, None)?;
        }
        Ok(())
    }

    pub fn op_ifelse(&mut self) -> Result<(), String> {
        let proc_false = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => {
                return Err(format!(
                    "ifelse: expected procedure for false branch, got {:?}",
                    other
                ));
            }
        };
        let proc_true = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => {
                return Err(format!(
                    "ifelse: expected procedure for true branch, got {:?}",
                    other
                ));
            }
        };
        let condition = match self.stack.pop()? {
            Value::Bool(b) => b,
            other => return Err(format!("ifelse: expected bool condition, got {:?}", other)),
        };
        if condition {
            self.execute_procedure(&proc_true, None)?;
        } else {
            self.execute_procedure(&proc_false, None)?;
        }
        Ok(())
    }

    pub fn op_for(&mut self) -> Result<(), String> {
        let proc = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => return Err(format!("for: expected procedure, got {:?}", other)),
        };
        let limit = match self.stack.pop()? {
            Value::Int(n) => n as f64,
            Value::Float(f) => f,
            other => return Err(format!("for: expected numeric limit, got {:?}", other)),
        };
        let increment = match self.stack.pop()? {
            Value::Int(n) => n as f64,
            Value::Float(f) => f,
            other => return Err(format!("for: expected numeric increment, got {:?}", other)),
        };
        let init = match self.stack.pop()? {
            Value::Int(n) => n as f64,
            Value::Float(f) => f,
            other => return Err(format!("for: expected numeric init, got {:?}", other)),
        };

        let use_float = init.fract() != 0.0 || increment.fract() != 0.0 || limit.fract() != 0.0;

        if increment == 0.0 {
            return Err("for: increment cannot be zero".to_string());
        }

        let mut counter = init;
        loop {
            if increment > 0.0 && counter > limit {
                break;
            }
            if increment < 0.0 && counter < limit {
                break;
            }
            if use_float {
                self.stack.push(Value::Float(counter));
            } else {
                self.stack.push(Value::Int(counter as i64));
            }
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
            other => {
                return Err(format!(
                    "repeat: expected non-negative int, got {:?}",
                    other
                ));
            }
        };
        for _ in 0..n {
            self.execute_procedure(&proc, None)?;
        }
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

    fn run_err(source: &str) -> String {
        let mut interp = Interpreter::new();
        interp.run(source).unwrap_err()
    }

    // ── if ────────────────────────────────────────────────────────────────────

    #[test]
    fn test_if_true() {
        assert_eq!(run("true { 42 } if"), vec![Value::Int(42)]);
    }

    #[test]
    fn test_if_false() {
        assert_eq!(run("false { 42 } if"), vec![]);
    }

    #[test]
    fn test_if_wrong_condition_type() {
        let mut interp = Interpreter::new();
        assert!(interp.run("1 { 42 } if").is_err());
    }

    #[test]
    fn test_if_wrong_proc_type() {
        let mut interp = Interpreter::new();
        assert!(interp.run("true 42 if").is_err());
    }

    // ── ifelse ────────────────────────────────────────────────────────────────

    #[test]
    fn test_ifelse_true() {
        assert_eq!(run("true { 1 } { 2 } ifelse"), vec![Value::Int(1)]);
    }

    #[test]
    fn test_ifelse_false() {
        assert_eq!(run("false { 1 } { 2 } ifelse"), vec![Value::Int(2)]);
    }

    #[test]
    fn test_ifelse_wrong_false_branch() {
        let mut interp = Interpreter::new();
        assert!(interp.run("true { 1 } 42 ifelse").is_err());
    }

    #[test]
    fn test_ifelse_wrong_true_branch() {
        let mut interp = Interpreter::new();
        assert!(interp.run("true 42 { 2 } ifelse").is_err());
    }

    #[test]
    fn test_ifelse_wrong_condition() {
        let mut interp = Interpreter::new();
        assert!(interp.run("1 { 1 } { 2 } ifelse").is_err());
    }

    // ── for ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_for_basic() {
        assert_eq!(
            run("1 1 3 { } for"),
            vec![Value::Int(1), Value::Int(2), Value::Int(3)]
        );
    }

    #[test]
    fn test_for_with_add() {
        assert_eq!(run("0 1 1 3 { add } for"), vec![Value::Int(6)]);
    }

    #[test]
    fn test_for_countdown() {
        assert_eq!(
            run("3 -1 1 { } for"),
            vec![Value::Int(3), Value::Int(2), Value::Int(1)]
        );
    }

    #[test]
    fn test_for_float_counter() {
        // float init triggers use_float path
        let result = run("0.0 1.0 3.0 { } for");
        assert_eq!(
            result,
            vec![
                Value::Float(0.0),
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0)
            ]
        );
    }

    #[test]
    fn test_for_float_limit() {
        // float limit triggers use_float
        let result = run("0 1 2.5 { } for");
        assert_eq!(
            result,
            vec![Value::Float(0.0), Value::Float(1.0), Value::Float(2.0)]
        );
    }

    #[test]
    fn test_for_zero_increment_error() {
        assert!(Interpreter::new().run("0 0 10 { } for").is_err());
    }

    #[test]
    fn test_for_wrong_proc_type() {
        assert!(Interpreter::new().run("1 1 3 42 for").is_err());
    }

    #[test]
    fn test_for_wrong_limit_type() {
        assert!(Interpreter::new().run("1 1 (bad) { } for").is_err());
    }

    #[test]
    fn test_for_wrong_increment_type() {
        assert!(Interpreter::new().run("1 (bad) 3 { } for").is_err());
    }

    #[test]
    fn test_for_wrong_init_type() {
        assert!(Interpreter::new().run("(bad) 1 3 { } for").is_err());
    }

    // ── repeat ────────────────────────────────────────────────────────────────

    #[test]
    fn test_repeat() {
        assert_eq!(
            run("3 { 1 } repeat"),
            vec![Value::Int(1), Value::Int(1), Value::Int(1)]
        );
    }

    #[test]
    fn test_repeat_zero() {
        assert_eq!(run("0 { 99 } repeat"), vec![]);
    }

    #[test]
    fn test_repeat_wrong_proc_type() {
        assert!(Interpreter::new().run("3 42 repeat").is_err());
    }

    #[test]
    fn test_repeat_wrong_count_type() {
        assert!(Interpreter::new().run("(bad) { } repeat").is_err());
    }

    // ── quit ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_quit_returns_sentinel() {
        assert_eq!(run_err("quit"), "__quit__");
    }
}
