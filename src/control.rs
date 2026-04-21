// control.rs — Flow control operators
//
// Implements: if, ifelse, for, repeat, quit
//
// All flow control operators need access to both the operand stack AND
// the ability to execute procedures, so they are implemented as methods
// on Interpreter rather than on OperandStack.
//
// This keeps the execution logic centralized while still separating concerns.

use crate::interpreter::Interpreter;
use crate::types::Value;

impl Interpreter {

    /// if — bool proc →
    /// Execute proc only if bool is true.
    ///   Before: true { 42 }
    ///   After:  42
    pub fn op_if(&mut self) -> Result<(), String> {
        // Pop the procedure first (it's on top), then the condition
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

    /// ifelse — bool proc_true proc_false →
    /// Execute proc_true if bool is true, proc_false otherwise.
    ///   Before: true { 1 } { 2 }
    ///   After:  1
    pub fn op_ifelse(&mut self) -> Result<(), String> {
        // Stack order (top to bottom): proc_false, proc_true, bool
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

        if condition {
            self.execute_procedure(&proc_true, None)?;
        } else {
            self.execute_procedure(&proc_false, None)?;
        }
        Ok(())
    }

    /// for — init increment limit proc →
    /// Execute proc for each value from init to limit (inclusive), stepping by increment.
    /// The current counter value is pushed onto the stack before each execution.
    ///
    ///   Before: 1 1 5 { = } for
    ///   Prints: 1 2 3 4 5
    ///
    /// Works with both integers and floats.
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

        // Determine whether we're working with integers or floats
        // (all three control values must be checked)
        let use_float = matches!(
            (self.stack.peek(), increment, limit),
            _ if init.fract() != 0.0 || increment.fract() != 0.0 || limit.fract() != 0.0
        );

        // Guard against infinite loops from zero increment
        if increment == 0.0 {
            return Err("for: increment cannot be zero".to_string());
        }

        let mut counter = init;

        // Loop condition depends on direction of increment
        loop {
            if increment > 0.0 && counter > limit { break; }
            if increment < 0.0 && counter < limit { break; }

            // Push the counter onto the stack before each iteration
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

    /// repeat — int proc →
    /// Execute proc exactly n times. The counter is NOT pushed onto the stack.
    ///   Before: 3 { (hi) print } repeat
    ///   Prints: hihihi
    pub fn op_repeat(&mut self) -> Result<(), String> {
        let proc = match self.stack.pop()? {
            Value::Procedure { tokens, .. } => tokens,
            other => return Err(format!("repeat: expected procedure, got {:?}", other)),
        };
        let n = match self.stack.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("repeat: expected non-negative int, got {:?}", other)),
        };

        for _ in 0..n {
            self.execute_procedure(&proc, None)?;
        }
        Ok(())
    }

    /// quit — terminate the interpreter by returning a special sentinel error.
    /// The REPL in main.rs catches this specific message and exits cleanly.
    pub fn op_quit(&mut self) -> Result<(), String> {
        Err("__quit__".to_string())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

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
    fn test_if_true() {
        assert_eq!(run("true { 42 } if"), vec![Value::Int(42)]);
    }

    #[test]
    fn test_if_false() {
        // When condition is false, nothing is pushed
        assert_eq!(run("false { 42 } if"), vec![]);
    }

    #[test]
    fn test_ifelse_true() {
        assert_eq!(run("true { 1 } { 2 } ifelse"), vec![Value::Int(1)]);
    }

    #[test]
    fn test_ifelse_false() {
        assert_eq!(run("false { 1 } { 2 } ifelse"), vec![Value::Int(2)]);
    }

    #[test]
    fn test_for_basic() {
        // 1 1 3 for: counter values 1, 2, 3 pushed onto stack
        assert_eq!(
            run("1 1 3 { } for"),
            vec![Value::Int(1), Value::Int(2), Value::Int(3)]
        );
    }

    #[test]
    fn test_for_with_add() {
        // Sum 1+2+3 = 6
        assert_eq!(run("0 1 1 3 { add } for"), vec![Value::Int(6)]);
    }

    #[test]
    fn test_for_countdown() {
        // Count down: 3 -1 1 → pushes 3, 2, 1
        assert_eq!(
            run("3 -1 1 { } for"),
            vec![Value::Int(3), Value::Int(2), Value::Int(1)]
        );
    }

    #[test]
    fn test_repeat() {
        // Push 1 three times
        assert_eq!(
            run("3 { 1 } repeat"),
            vec![Value::Int(1), Value::Int(1), Value::Int(1)]
        );
    }

    #[test]
    fn test_repeat_zero() {
        // Zero repeats — nothing happens
        assert_eq!(run("0 { 99 } repeat"), vec![]);
    }

    #[test]
    fn test_nested_if_in_for() {
        // Push even numbers from 1..4: 2, 4
        assert_eq!(
            run("1 1 4 { dup 2 mod 0 eq { } { pop } ifelse } for"),
            vec![Value::Int(2), Value::Int(4)]
        );
    }

    #[test]
    fn test_quit_returns_sentinel() {
        let mut interp = Interpreter::new();
        let result = interp.run("quit");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "__quit__");
    }
}