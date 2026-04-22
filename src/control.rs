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

        let (limit, limit_is_float) = match self.stack.pop()? {
            Value::Int(n) => (n as f64, false),
            Value::Float(f) => (f, true),
            other => return Err(format!("for: expected numeric limit, got {:?}", other)),
        };

        let (increment, increment_is_float) = match self.stack.pop()? {
            Value::Int(n) => (n as f64, false),
            Value::Float(f) => (f, true),
            other => return Err(format!("for: expected numeric increment, got {:?}", other)),
        };

        let (init, init_is_float) = match self.stack.pop()? {
            Value::Int(n) => (n as f64, false),
            Value::Float(f) => (f, true),
            other => return Err(format!("for: expected numeric init, got {:?}", other)),
        };

        let use_float = init_is_float || increment_is_float || limit_is_float;

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
