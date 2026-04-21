// interpreter.rs — PostScript Interpreter
//
// Responsibility: walk a Vec<Token> produced by the lexer and execute each
// token against the operand stack and dictionary stack.
//
// This file grows with each step. Right now (Step 3) it contains:
//   - The Value enum  (runtime types that live on the stack)
//   - The Interpreter struct  (operand stack + execution engine)
//   - Stack manipulation commands: exch, pop, dup, copy, clear, count
 
use crate::lexer::{Token, tokenize};
use std::collections::HashMap;
use std::fmt;
 
// ── Runtime value type ────────────────────────────────────────────────────────
 
/// Every value that can live on the PostScript operand stack.
///
/// PostScript is dynamically typed — the stack can hold a mix of these at once.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A whole number, e.g. 42
    Int(i64),
 
    /// A floating-point number, e.g. 3.14
    Float(f64),
 
    /// A boolean: true or false
    Bool(bool),
 
    /// A string, e.g. (hello)
    Str(String),
 
    /// A name (literal), e.g. /foo — stored as data, not executed
    Name(String),
 
    /// A procedure — a list of tokens to be executed later.
    /// Created when the lexer sees { ... }.
    Procedure(Vec<Token>),
}
 
/// Display formatting for Value — used by the `=` and `==` operators later.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n)       => write!(f, "{}", n),
            Value::Float(n)     => write!(f, "{}", n),
            Value::Bool(b)      => write!(f, "{}", b),
            Value::Str(s)       => write!(f, "{}", s),
            Value::Name(n)      => write!(f, "{}", n),
            // Procedures display as { ... } for debugging
            Value::Procedure(tokens) => {
                write!(f, "{{ ")?;
                for t in tokens {
                    write!(f, "{:?} ", t)?;
                }
                write!(f, "}}")
            }
        }
    }
}
 
// ── Interpreter ───────────────────────────────────────────────────────────────
 
/// The PostScript interpreter state.
///
/// Fields added in later steps:
///   - dict_stack  (Step 5)
///   - use_lexical_scoping flag  (Step 10)
pub struct Interpreter {
    /// The operand stack — the heart of PostScript.
    /// Values are pushed and popped as commands execute.
    /// The TOP of the stack is the LAST element of the Vec.
    pub operand_stack: Vec<Value>,
 
    /// The dictionary stack — added in Step 5.
    /// Declared here now so the struct is ready for it.
    pub dict_stack: Vec<HashMap<String, Value>>,
 
    /// Scoping mode flag — toggled in Step 10.
    /// false = dynamic scoping (PostScript default)
    /// true  = lexical (static) scoping
    pub use_lexical_scoping: bool,
}
 
impl Interpreter {
    /// Create a new interpreter with an empty operand stack.
    pub fn new() -> Self {
        Interpreter {
            operand_stack: Vec::new(),
            dict_stack: Vec::new(),
            use_lexical_scoping: false,
        }
    }
 
    /// Main entry point: tokenize a source string and execute every token.
    pub fn run(&mut self, source: &str) -> Result<(), String> {
        let tokens = tokenize(source)?;
        // We use an index rather than an iterator so that flow-control
        // operators (if, for, etc.) can manipulate the position later.
        let mut i = 0;
        while i < tokens.len() {
            self.execute_token(&tokens, &mut i)?;
            i += 1;
        }
        Ok(())
    }
 
    /// Execute a single token from the token stream.
    ///
    /// - Literal values (Int, Float, Bool, StringLit, LiteralName) are pushed
    ///   directly onto the operand stack.
    /// - ProcStart collects tokens until ProcEnd and pushes a Procedure value.
    /// - Name tokens are dispatched to execute_operator (or looked up in the
    ///   dict stack in a later step).
    fn execute_token(&mut self, tokens: &[Token], i: &mut usize) -> Result<(), String> {
        match &tokens[*i] {
            // ── Push literal values straight onto the stack ───────────────
            Token::Int(n)         => self.operand_stack.push(Value::Int(*n)),
            Token::Float(f)       => self.operand_stack.push(Value::Float(*f)),
            Token::Bool(b)        => self.operand_stack.push(Value::Bool(*b)),
            Token::StringLit(s)   => self.operand_stack.push(Value::Str(s.clone())),
            Token::LiteralName(n) => self.operand_stack.push(Value::Name(n.clone())),
 
            // ── Procedure: collect tokens between { and } ─────────────────
            // We advance `i` past all the inner tokens so the main loop
            // doesn't try to execute them individually.
            Token::ProcStart => {
                let proc_tokens = self.collect_procedure(tokens, i)?;
                self.operand_stack.push(Value::Procedure(proc_tokens));
            }
 
            // ── ProcEnd should never be seen here (consumed by ProcStart) ─
            Token::ProcEnd => {
                return Err("Unexpected '}'".to_string());
            }
 
            // ── Named token: built-in operator or user-defined name ───────
            Token::Name(name) => {
                let name = name.clone();
                self.execute_operator(&name, tokens, i)?;
            }
        }
        Ok(())
    }
 
    /// Collect all tokens between the current `{` and its matching `}`.
    ///
    /// Handles nested procedures by tracking brace depth.
    /// On return, `i` points at the closing `}`.
    fn collect_procedure(&self, tokens: &[Token], i: &mut usize) -> Result<Vec<Token>, String> {
        let mut proc_tokens = Vec::new();
        let mut depth = 1usize; // we already consumed the opening {
        *i += 1;               // move past the ProcStart
 
        while *i < tokens.len() {
            match &tokens[*i] {
                Token::ProcStart => {
                    depth += 1;
                    proc_tokens.push(tokens[*i].clone());
                }
                Token::ProcEnd => {
                    depth -= 1;
                    if depth == 0 {
                        // `i` now points at the closing }, main loop will +1
                        return Ok(proc_tokens);
                    }
                    proc_tokens.push(tokens[*i].clone());
                }
                other => {
                    proc_tokens.push(other.clone());
                }
            }
            *i += 1;
        }
 
        Err("Unterminated procedure — missing '}'".to_string())
    }
 
    /// Execute a procedure value (a Vec<Token>) by running each token in it.
    /// This is called by if, ifelse, for, repeat, etc. in later steps.
    pub fn execute_procedure(&mut self, proc_tokens: &[Token]) -> Result<(), String> {
        let mut i = 0;
        while i < proc_tokens.len() {
            self.execute_token(proc_tokens, &mut i)?;
            i += 1;
        }
        Ok(())
    }
 
    /// Dispatch a named token to the correct built-in operator.
    ///
    /// Steps add more arms to this match as new commands are implemented.
    fn execute_operator(&mut self, name: &str, _tokens: &[Token], _i: &mut usize) -> Result<(), String> {
        match name {
            // ── Stack manipulation (Step 3) ───────────────────────────────
            "exch"  => self.op_exch(),
            "pop"   => self.op_pop(),
            "dup"   => self.op_dup(),
            "copy"  => self.op_copy(),
            "clear" => self.op_clear(),
            "count" => self.op_count(),
 
            // ── Unknown — will be extended in future steps ─────────────
            _ => Err(format!("Unknown operator: {}", name)),
        }
    }
 
    // ── Stack helpers ─────────────────────────────────────────────────────────
 
    /// Pop one value off the stack, returning an error if the stack is empty.
    fn pop(&mut self) -> Result<Value, String> {
        self.operand_stack.pop()
            .ok_or_else(|| "Stack underflow".to_string())
    }
 
    /// Peek at the top value without removing it.
    #[allow(dead_code)]
    fn peek(&self) -> Result<&Value, String> {
        self.operand_stack.last()
            .ok_or_else(|| "Stack underflow".to_string())
    }
 
    // ── Stack manipulation operators ──────────────────────────────────────────
 
    /// exch — swap the top two stack elements
    ///   Before: a b      (b is on top)
    ///   After:  b a      (a is now on top)
    fn op_exch(&mut self) -> Result<(), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.operand_stack.push(b);
        self.operand_stack.push(a);
        Ok(())
    }
 
    /// pop — discard the top element
    ///   Before: a
    ///   After:  (empty)
    fn op_pop(&mut self) -> Result<(), String> {
        self.pop()?;
        Ok(())
    }
 
    /// dup — duplicate the top element
    ///   Before: a
    ///   After:  a a
    fn op_dup(&mut self) -> Result<(), String> {
        let top = self.peek()?.clone();
        self.operand_stack.push(top);
        Ok(())
    }
 
    /// copy — duplicate the top n elements in order
    ///   Before: a b c  3
    ///   After:  a b c  a b c
    fn op_copy(&mut self) -> Result<(), String> {
        // The top of the stack must be an integer n
        let n = match self.pop()? {
            Value::Int(n) if n >= 0 => n as usize,
            other => return Err(format!("copy: expected non-negative int, got {:?}", other)),
        };
 
        let len = self.operand_stack.len();
        if n > len {
            return Err(format!("copy: requested {} elements but stack only has {}", n, len));
        }
 
        // Clone the top n elements and append them
        let top_n: Vec<Value> = self.operand_stack[len - n..].to_vec();
        self.operand_stack.extend(top_n);
        Ok(())
    }
 
    /// clear — remove all elements from the stack
    ///   Before: a b c ...
    ///   After:  (empty)
    fn op_clear(&mut self) -> Result<(), String> {
        self.operand_stack.clear();
        Ok(())
    }
 
    /// count — push the number of elements currently on the stack
    ///   Before: a b c      (3 elements)
    ///   After:  a b c  3
    fn op_count(&mut self) -> Result<(), String> {
        let n = self.operand_stack.len() as i64;
        self.operand_stack.push(Value::Int(n));
        Ok(())
    }
}
 
// ── Unit tests ────────────────────────────────────────────────────────────────
 
#[cfg(test)]
mod tests {
    use super::*;
 
    // Helper: run PostScript source and return the operand stack
    fn run(source: &str) -> Vec<Value> {
        let mut interp = Interpreter::new();
        interp.run(source).expect("interpreter error");
        interp.operand_stack
    }
 
    #[test]
    fn test_push_int() {
        assert_eq!(run("1 2 3"), vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    }
 
    #[test]
    fn test_push_float() {
        assert_eq!(run("3.14"), vec![Value::Float(3.14)]);
    }
 
    #[test]
    fn test_push_bool() {
        assert_eq!(run("true false"), vec![Value::Bool(true), Value::Bool(false)]);
    }
 
    #[test]
    fn test_push_string() {
        assert_eq!(run("(hello)"), vec![Value::Str("hello".to_string())]);
    }
 
    #[test]
    fn test_push_literal_name() {
        assert_eq!(run("/foo"), vec![Value::Name("foo".to_string())]);
    }
 
    #[test]
    fn test_push_procedure() {
        let stack = run("{ 1 2 }");
        assert_eq!(stack.len(), 1);
        // Just confirm a Procedure was pushed
        assert!(matches!(stack[0], Value::Procedure(_)));
    }
 
    #[test]
    fn test_exch() {
        assert_eq!(run("1 2 exch"), vec![Value::Int(2), Value::Int(1)]);
    }
 
    #[test]
    fn test_pop() {
        assert_eq!(run("1 2 pop"), vec![Value::Int(1)]);
    }
 
    #[test]
    fn test_dup() {
        assert_eq!(run("5 dup"), vec![Value::Int(5), Value::Int(5)]);
    }
 
    #[test]
    fn test_copy() {
        // copy 2: duplicate top 2 elements
        assert_eq!(
            run("1 2 3 2 copy"),
            vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(2), Value::Int(3)]
        );
    }
 
    #[test]
    fn test_clear() {
        assert_eq!(run("1 2 3 clear"), vec![]);
    }
 
    #[test]
    fn test_count() {
        assert_eq!(run("1 2 3 count"), vec![
            Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(3)
        ]);
    }
 
    #[test]
    fn test_stack_underflow() {
        let mut interp = Interpreter::new();
        assert!(interp.run("pop").is_err());
    }
}