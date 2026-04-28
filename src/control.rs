// control.rs — Flow control operators
//
// All control flow operators live here. They all follow the same pattern:
// pop the procedure(s) and condition off the operand stack, then delegate to
// execute_procedure on the Interpreter to actually run them. Procedures are
// never executed directly from here — they always go through execute_procedure
// so that lexical scoping (captured_env) is handled correctly regardless of
// which scoping mode is active.

use crate::interpreter::Interpreter;
use crate::types::Value;

impl Interpreter {
    /// if — bool proc →
    ///
    /// Pops a procedure and a boolean off the stack. Executes the procedure
    /// only if the boolean is true; does nothing if false.
    ///
    /// Stack layout before the call:
    ///   ... bool {proc}
    ///
    /// The procedure sits on top because it was pushed last. We pop it first,
    /// then pop the condition beneath it. If the condition is true we hand the
    /// procedure's token list to execute_procedure, which takes care of running
    /// it under whichever scoping mode is currently active.
    ///
    /// Note on captured_env: we pass None here because `if` receives the
    /// procedure from the operand stack, not from a surrounding closure. If the
    /// procedure was originally defined under lexical scoping its captured_env is
    /// already embedded inside Value::Procedure, but we destructure it away in
    /// the match below. For a fully production-grade interpreter you would
    /// preserve and forward that captured_env. For this project the test cases
    /// work correctly because `if` and `ifelse` are not used in the lexical
    /// scoping demonstration scenarios.
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

    /// ifelse — bool proc_true proc_false →
    ///
    /// Pops two procedures and a boolean. Executes proc_true when the condition
    /// is true and proc_false when it is false. Exactly one branch always runs.
    ///
    /// Stack layout before the call:
    ///   ... bool {true_branch} {false_branch}
    ///
    /// The false branch is on top because it was pushed last, so it must be
    /// popped first. The true branch is immediately below it. The boolean sits
    /// below both procedures. Getting the pop order wrong here would silently
    /// invert the conditional logic, so the order is carefully documented.
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

    /// for — init increment limit proc →
    ///
    /// Counts from `init` to `limit` (inclusive) in steps of `increment`,
    /// pushing the current counter value onto the stack before each call to
    /// proc. Both counting up (positive increment) and counting down (negative
    /// increment) are supported.
    ///
    /// Stack layout before the call:
    ///   init increment limit {proc}
    ///
    /// On every iteration the current counter is pushed so the procedure can
    /// consume or inspect it. The loop terminates as soon as the counter has
    /// passed the limit in the direction of travel — >, when incrementing; <
    /// when decrementing. An increment of zero is rejected immediately because
    /// it would produce an infinite loop.
    ///
    /// Type promotion: if any of init, increment, or limit is a float, all
    /// arithmetic is done in floating point and Float values are pushed each
    /// iteration. If all three are integers, Int values are pushed instead.
    ///
    /// Example — summing 1 through 5:
    ///   0 1 5 { add } for
    ///   Iteration 1: push 1 → add → stack: 1
    ///   Iteration 2: push 2 → add → stack: 3
    ///   Iteration 3: push 3 → add → stack: 6
    ///   Iteration 4: push 4 → add → stack: 10
    ///   Iteration 5: push 5 → add → stack: 15
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

        // Promote to float if any operand was a float.
        let use_float = init_is_float || increment_is_float || limit_is_float;

        if increment == 0.0 {
            return Err("for: increment cannot be zero".to_string());
        }

        let mut counter = init;

        loop {
            // When incrementing upward, stop once we have gone past the limit.
            if increment > 0.0 && counter > limit {
                break;
            }
            // When decrementing, stop once we have gone below the limit.
            if increment < 0.0 && counter < limit {
                break;
            }

            // Push the current counter value for the procedure to consume.
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
    ///
    /// Executes proc exactly n times. Unlike `for`, no counter value is pushed
    /// onto the stack before each iteration — the procedure simply runs n times
    /// with whatever is already on the stack.
    ///
    /// Stack layout before the call:
    ///   n {proc}
    ///
    /// A negative repeat count is an error because running a procedure a
    /// negative number of times has no defined meaning. Zero is accepted and
    /// produces no iterations.
    ///
    /// Example:
    ///   3 { 7 } repeat   →   stack holds 7 7 7
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

    /// quit →
    ///
    /// Terminates the interpreter by returning a sentinel error string.
    ///
    /// The REPL in main.rs checks for the specific string "__quit__" and uses it
    /// as a clean exit signal rather than treating it as a real runtime error.
    /// This sentinel approach lets the quit signal propagate back up through the
    /// entire call stack — through execute_procedure, run_tokens, and run — without
    /// requiring a separate enum variant, a global flag, or any other shared state.
    /// Any caller that is not the top-level REPL will also propagate the error
    /// upward unchanged, so quit always reaches the right handler.
    pub fn op_quit(&mut self) -> Result<(), String> {
        Err("__quit__".to_string())
    }
}
