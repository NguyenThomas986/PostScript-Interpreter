// interpreter.rs — Core execution engine
//
// Responsibility:
//   - Tokenize source via the lexer
//   - Dispatch each token to the right module (stack, arithmetic, etc.)
//   - Manage the dictionary stack (added Step 5)
//   - Handle procedure collection and execution
//   - Hold the scoping mode flag (Step 10)
//
// This file stays thin — operator logic lives in stack.rs, arithmetic.rs, etc.

use crate::lexer::{Token, tokenize};
use crate::stack::OperandStack;
use crate::types::Value;
use std::collections::HashMap;

pub struct Interpreter {
    /// The operand stack — owned by OperandStack in stack.rs
    pub stack: OperandStack,

    /// The dictionary stack — each entry is a scope (HashMap of name → Value)
    /// Top of the Vec = current (innermost) scope
    pub dict_stack: Vec<HashMap<String, Value>>,

    /// false = dynamic scoping (PostScript default)
    /// true  = lexical (static) scoping  — toggled in Step 10
    pub use_lexical_scoping: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            stack: OperandStack::new(),
            dict_stack: Vec::new(),
            use_lexical_scoping: false,
        }
    }

    /// Tokenize and execute a PostScript source string
    pub fn run(&mut self, source: &str) -> Result<(), String> {
        let tokens = tokenize(source)?;
        let mut i = 0;
        while i < tokens.len() {
            self.execute_token(&tokens, &mut i)?;
            i += 1;
        }
        Ok(())
    }

    /// Execute a single token
    pub fn execute_token(&mut self, tokens: &[Token], i: &mut usize) -> Result<(), String> {
        match &tokens[*i] {
            Token::Int(n)         => self.stack.push(Value::Int(*n)),
            Token::Float(f)       => self.stack.push(Value::Float(*f)),
            Token::Bool(b)        => self.stack.push(Value::Bool(*b)),
            Token::StringLit(s)   => self.stack.push(Value::Str(s.clone())),
            Token::LiteralName(n) => self.stack.push(Value::Name(n.clone())),

            Token::ProcStart => {
                let proc_tokens = self.collect_procedure(tokens, i)?;
                self.stack.push(Value::Procedure(proc_tokens));
            }

            Token::ProcEnd => return Err("Unexpected '}'".to_string()),

            Token::Name(name) => {
                let name = name.clone();
                self.dispatch(&name, tokens, i)?;
            }
        }
        Ok(())
    }

    /// Collect tokens between { and matching } into a Procedure value
    fn collect_procedure(&self, tokens: &[Token], i: &mut usize) -> Result<Vec<Token>, String> {
        let mut proc_tokens = Vec::new();
        let mut depth = 1usize;
        *i += 1;

        while *i < tokens.len() {
            match &tokens[*i] {
                Token::ProcStart => { depth += 1; proc_tokens.push(tokens[*i].clone()); }
                Token::ProcEnd   => {
                    depth -= 1;
                    if depth == 0 { return Ok(proc_tokens); }
                    proc_tokens.push(tokens[*i].clone());
                }
                other => proc_tokens.push(other.clone()),
            }
            *i += 1;
        }
        Err("Unterminated procedure — missing '}'".to_string())
    }

    /// Execute a procedure (Vec<Token>) — called by if, ifelse, for, repeat
    pub fn execute_procedure(&mut self, proc_tokens: &[Token]) -> Result<(), String> {
        let mut i = 0;
        while i < proc_tokens.len() {
            self.execute_token(proc_tokens, &mut i)?;
            i += 1;
        }
        Ok(())
    }

    /// Dispatch a named token to the correct operator implementation
    fn dispatch(&mut self, name: &str, _tokens: &[Token], _i: &mut usize) -> Result<(), String> {
        match name {
            // ── Stack manipulation (stack.rs) ─────────────────────────────
            "exch"  => self.stack.op_exch(),
            "pop"   => self.stack.op_pop(),
            "dup"   => self.stack.op_dup(),
            "copy"  => self.stack.op_copy(),
            "clear" => self.stack.op_clear(),
            "count" => self.stack.op_count(),

            // ── Arithmetic (arithmetic.rs) ────────────────────────────────
            "add"     => self.stack.op_add(),
            "sub"     => self.stack.op_sub(),
            "mul"     => self.stack.op_mul(),
            "div"     => self.stack.op_div(),
            "idiv"    => self.stack.op_idiv(),
            "mod"     => self.stack.op_mod(),
            "abs"     => self.stack.op_abs(),
            "neg"     => self.stack.op_neg(),
            "ceiling" => self.stack.op_ceiling(),
            "floor"   => self.stack.op_floor(),
            "round"   => self.stack.op_round(),
            "sqrt"    => self.stack.op_sqrt(),

            // ── Scoping toggle ────────────────────────────────────────────
            "lexical" => { self.use_lexical_scoping = true;  Ok(()) }
            "dynamic" => { self.use_lexical_scoping = false; Ok(()) }

            // ── Unknown ───────────────────────────────────────────────────
            _ => Err(format!("Unknown operator: {}", name)),
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source: &str) -> Vec<Value> {
        let mut interp = Interpreter::new();
        interp.run(source).expect("interpreter error");
        interp.stack.as_slice().to_vec()
    }

    #[test]
    fn test_push_values() {
        assert_eq!(run("1 2 3"), vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    }

    #[test]
    fn test_procedure_pushed() {
        let stack = run("{ 1 2 add }");
        assert!(matches!(stack[0], Value::Procedure(_)));
    }

    #[test]
    fn test_arithmetic_dispatch() {
        assert_eq!(run("3 4 add"), vec![Value::Int(7)]);
    }

    #[test]
    fn test_stack_dispatch() {
        assert_eq!(run("1 2 exch"), vec![Value::Int(2), Value::Int(1)]);
    }

    #[test]
    fn test_unknown_operator() {
        let mut interp = Interpreter::new();
        assert!(interp.run("foobar").is_err());
    }
}