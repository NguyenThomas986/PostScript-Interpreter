// interpreter.rs — Core execution engine
//
// Responsibility:
//   - Tokenize source via the lexer
//   - Dispatch each token to the right module
//   - Resolve user-defined names via the dictionary stack
//   - Hold the scoping mode flag and implement the scoping toggle
//
// Step 10: Lexical scoping is implemented here.
//   - Under dynamic scoping (default), procedures look up names in the
//     live dict stack at call time.
//   - Under lexical scoping, when a { } block is collected, we snapshot
//     the current dict stack and store it inside the Procedure value.
//     When the procedure runs, execute_procedure temporarily swaps in
//     that snapshot so all name lookups resolve against the definition-time
//     environment instead of the call-time environment.

use crate::lexer::{Token, tokenize};
use crate::stack::OperandStack;
use crate::dictionary::{Dict, DictStack};
use crate::types::Value;

pub struct Interpreter {
    /// The operand stack
    pub stack: OperandStack,

    /// The dictionary stack — manages all name bindings and scoping
    pub dicts: DictStack,

    /// false = dynamic scoping (PostScript default)
    /// true  = lexical (static) scoping
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
            Token::Int(n)         => self.stack.push(Value::Int(*n)),
            Token::Float(f)       => self.stack.push(Value::Float(*f)),
            Token::Bool(b)        => self.stack.push(Value::Bool(*b)),
            Token::StringLit(s)   => self.stack.push(Value::Str(s.clone())),
            Token::LiteralName(n) => self.stack.push(Value::Name(n.clone())),

            Token::ProcStart => {
                let proc_tokens = self.collect_procedure(tokens, i)?;

                // Under lexical scoping, snapshot the dict stack now
                // (at definition time) so the procedure carries its own
                // closed-over environment with it.
                let captured_env = if self.use_lexical_scoping {
                    Some(self.dicts.snapshot())
                } else {
                    None
                };

                self.stack.push(Value::Procedure { tokens: proc_tokens, captured_env });
            }

            Token::ProcEnd => return Err("Unexpected '}'".to_string()),

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

    /// Execute a procedure's token list.
    ///
    /// If the procedure carries a captured environment (lexical scoping),
    /// we temporarily swap the dict stack for that snapshot so all name
    /// lookups inside the procedure resolve against the definition-time env.
    /// After the procedure finishes, the live dict stack is restored.
    pub fn execute_procedure(&mut self, proc_tokens: &[Token], captured_env: Option<Vec<Dict>>) -> Result<(), String> {
        if let Some(env) = captured_env {
            // Save the current live dict stack
            let live_stack = self.dicts.swap(env);

            // Run the procedure in the captured environment
            let result = self.run_tokens(proc_tokens);

            // Always restore the live stack, even if there was an error
            self.dicts.swap(live_stack);

            result
        } else {
            // Dynamic scoping — just run against the current live stack
            self.run_tokens(proc_tokens)
        }
    }

    /// Internal helper: run a token slice without touching the dict stack
    fn run_tokens(&mut self, tokens: &[Token]) -> Result<(), String> {
        let mut i = 0;
        while i < tokens.len() {
            self.execute_token(tokens, &mut i)?;
            i += 1;
        }
        Ok(())
    }

    /// Dispatch a name to a built-in operator or user-defined name
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
            "maxlength" => self.dicts.op_maxlength(&mut self.stack),
            "begin"     => self.dicts.op_begin(&mut self.stack),
            "end"       => self.dicts.op_end(),
            "def"       => self.dicts.op_def(&mut self.stack),

            // ── length: routes based on top-of-stack type ─────────────────
            "length" => {
                match self.stack.peek()? {
                    Value::Str(_)  => self.stack.op_string_length(),
                    Value::Dict(_) => self.dicts.op_length(&mut self.stack),
                    other => Err(format!("length: expected string or dict, got {:?}", other)),
                }
            }

            // ── String operators (strings.rs) ─────────────────────────────
            "get"         => self.stack.op_get(),
            "getinterval" => self.stack.op_getinterval(),
            "putinterval" => self.stack.op_putinterval(),

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

            // ── I/O operators (io_ops.rs) ─────────────────────────────────
            "print" => self.stack.op_print(),
            "="     => self.stack.op_print_pop(),
            "=="    => self.stack.op_print_repr(),

            // ── Flow control (control.rs) ─────────────────────────────────
            "if"     => self.op_if(),
            "ifelse" => self.op_ifelse(),
            "for"    => self.op_for(),
            "repeat" => self.op_repeat(),
            "quit"   => self.op_quit(),

            // ── Scoping toggle ────────────────────────────────────────────
            "lexical" => { self.use_lexical_scoping = true;  Ok(()) }
            "dynamic" => { self.use_lexical_scoping = false; Ok(()) }

            // ── User-defined name: look up in dictionary stack ────────────
            _ => {
                match self.dicts.lookup(name) {
                    Some(Value::Procedure { tokens, captured_env }) => {
                        let tokens = tokens.clone();
                        let env = captured_env.clone();
                        self.execute_procedure(&tokens, env)
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
        assert_eq!(run("/x 42 def  x"), vec![Value::Int(42)]);
    }

    #[test]
    fn test_def_procedure_and_call() {
        assert_eq!(run("/inc { 1 add } def  5 inc"), vec![Value::Int(6)]);
    }

    #[test]
    fn test_begin_end_scope() {
        let result = run("
            /x 1 def
            10 dict begin
                /x 99 def
                x
            end
            x
        ");
        assert_eq!(result, vec![Value::Int(99), Value::Int(1)]);
    }

    #[test]
    fn test_dynamic_scoping_default() {
        // Under dynamic scoping, getx sees the REDEFINED x = 99
        let result = run("
            /x 10 def
            /getx { x } def
            /x 99 def
            getx
        ");
        assert_eq!(result, vec![Value::Int(99)]);
    }

    #[test]
    fn test_lexical_scoping_captures_definition_env() {
        // Under lexical scoping, getx sees x = 10 from when it was defined
        let result = run("
            lexical
            /x 10 def
            /getx { x } def
            /x 99 def
            getx
        ");
        assert_eq!(result, vec![Value::Int(10)]);
    }

    #[test]
    fn test_dynamic_after_lexical_toggle() {
        // Switch back to dynamic — should see redefined x = 99 again
        let result = run("
            lexical
            dynamic
            /x 10 def
            /getx { x } def
            /x 99 def
            getx
        ");
        assert_eq!(result, vec![Value::Int(99)]);
    }

    #[test]
    fn test_unknown_name_errors() {
        let mut interp = Interpreter::new();
        assert!(interp.run("notdefined").is_err());
    }
}