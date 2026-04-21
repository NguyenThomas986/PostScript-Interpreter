// interpreter.rs — Core execution engine
//
// Responsibility:
//   - Tokenize source via the lexer
//   - Dispatch each token to the right module
//   - Resolve user-defined names via the dictionary stack
//   - Hold the scoping mode flag (Step 10)
//
// Operator logic lives in stack.rs, arithmetic.rs, dictionary.rs etc.

use crate::lexer::{Token, tokenize};
use crate::stack::OperandStack;
use crate::dictionary::DictStack;
use crate::types::Value;

pub struct Interpreter {
    /// The operand stack
    pub stack: OperandStack,

    /// The dictionary stack — manages all name bindings and scoping
    pub dicts: DictStack,

    /// false = dynamic scoping (PostScript default)
    /// true  = lexical (static) scoping — toggled in Step 10
    pub use_lexical_scoping: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            stack: OperandStack::new(),
            dicts: DictStack::new(),
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
            // ── Literal values: push straight onto the operand stack ──────
            Token::Int(n)         => self.stack.push(Value::Int(*n)),
            Token::Float(f)       => self.stack.push(Value::Float(*f)),
            Token::Bool(b)        => self.stack.push(Value::Bool(*b)),
            Token::StringLit(s)   => self.stack.push(Value::Str(s.clone())),
            Token::LiteralName(n) => self.stack.push(Value::Name(n.clone())),

            // ── Procedure: collect { ... } into a Procedure value ─────────
            Token::ProcStart => {
                let proc_tokens = self.collect_procedure(tokens, i)?;
                self.stack.push(Value::Procedure(proc_tokens));
            }

            Token::ProcEnd => return Err("Unexpected '}'".to_string()),

            // ── Named token: try built-ins first, then user dict lookup ───
            Token::Name(name) => {
                let name = name.clone();
                self.dispatch(&name, tokens, i)?;
            }
        }
        Ok(())
    }

    /// Collect tokens between { and matching } into a Vec<Token>
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

    /// Dispatch a name to a built-in operator, or look it up in the dict stack
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

            // ── Dictionary (dictionary.rs) ────────────────────────────────
            "dict"      => self.dicts.op_dict(&mut self.stack),
            "length"    => self.dicts.op_length(&mut self.stack),
            "maxlength" => self.dicts.op_maxlength(&mut self.stack),
            "begin"     => self.dicts.op_begin(&mut self.stack),
            "end"       => self.dicts.op_end(),
            "def"       => self.dicts.op_def(&mut self.stack),

            // ── Boolean & comparison (boolean.rs) ────────────────────────
            "eq"    => self.stack.op_eq(),
            "ne"    => self.stack.op_ne(),
            "ge"    => self.stack.op_ge(),
            "gt"    => self.stack.op_gt(),
            "le"    => self.stack.op_le(),
            "lt"    => self.stack.op_lt(),
            "and"   => self.stack.op_and(),
            "or"    => self.stack.op_or(),
            "not"   => self.stack.op_not(),
            "true"  => self.stack.op_true(),
            "false" => self.stack.op_false(),

            // ── Scoping toggle ────────────────────────────────────────────
            "lexical" => { self.use_lexical_scoping = true;  Ok(()) }
            "dynamic" => { self.use_lexical_scoping = false; Ok(()) }

            // ── User-defined name: look up in dictionary stack ────────────
            // If the resolved value is a Procedure, execute it.
            // Otherwise push it onto the operand stack.
            _ => {
                match self.dicts.lookup(name) {
                    Some(Value::Procedure(tokens)) => {
                        // Clone the tokens so we don't hold a borrow on self
                        let tokens = tokens.clone();
                        self.execute_procedure(&tokens)
                    }
                    Some(val) => {
                        self.stack.push(val);
                        Ok(())
                    }
                    None => Err(format!("Unknown name: {}", name)),
                }
            }
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
    fn test_def_and_lookup() {
        // Define x = 42, then push x — should get 42 on the stack
        assert_eq!(run("/x 42 def  x"), vec![Value::Int(42)]);
    }

    #[test]
    fn test_def_procedure_and_call() {
        // Define a procedure that adds 1, call it
        assert_eq!(run("/inc { 1 add } def  5 inc"), vec![Value::Int(6)]);
    }

    #[test]
    fn test_begin_end_scope() {
        // x defined globally, then shadowed inside a begin/end block
        let result = run("
            /x 1 def
            10 dict begin
                /x 99 def
                x        % should be 99 here
            end
            x            % should be 1 again here
        ");
        assert_eq!(result, vec![Value::Int(99), Value::Int(1)]);
    }

    #[test]
    fn test_dict_length() {
        // Create a dict, push it onto the dict stack, define two names,
        // then end — stack should be empty and nothing should crash
        let result = run("
            2 dict begin
            /a 1 def
            /b 2 def
            end
        ");
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_unknown_name_errors() {
        let mut interp = Interpreter::new();
        assert!(interp.run("notdefined").is_err());
    }
}