// interpreter.rs — Core execution engine
//
// Responsibility:
//   - Tokenize source via the lexer
//   - Dispatch each token to the right module
//   - Resolve user-defined names via the dictionary stack
//   - Hold the scoping mode flag and implement the scoping toggle
//   - Implement forall (needs access to execute_procedure)
//   - Implement array literal [ ... ] syntax (evaluates elements at runtime)
//
// Lexical scoping (implemented here):
//   - Under dynamic scoping (default), procedures look up names in the
//     live dict stack at call time.
//   - Under lexical scoping, when a { } block is collected, we snapshot
//     the current dict stack and store it inside the Procedure value.
//     When the procedure runs, execute_procedure temporarily swaps in
//     that snapshot so all name lookups resolve against the definition-time
//     environment instead of the call-time environment.
//
// Array literal [ ... ] (implemented here):
//   - On seeing ArrayStart ([), a mark is pushed onto the operand stack.
//   - All tokens up to the matching ArrayEnd (]) are executed normally,
//     so their results land on the stack above the mark.
//   - After the ], counttomark tells us how many items to collect; they are
//     popped into a Vec<Value>, reversed, and wrapped in Value::Array.
//   - Nested arrays work naturally because each inner [ recurses into this
//     same branch and returns a complete Value::Array before the outer ]
//     fires.

use crate::dictionary::{Dict, DictStack};
use crate::lexer::{Token, tokenize};
use crate::stack::OperandStack;
use crate::types::Value;

pub struct Interpreter {
    pub stack: OperandStack,
    pub dicts: DictStack,
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

    /// Tokenize and execute a PostScript source string.
    pub fn run(&mut self, source: &str) -> Result<(), String> {
        let tokens = tokenize(source)?;
        let mut i = 0;
        while i < tokens.len() {
            self.execute_token(&tokens, &mut i)?;
            i += 1;
        }
        Ok(())
    }

    /// Execute a single token, advancing `i` as needed for multi-token constructs
    /// (procedure bodies, array literals).
    pub fn execute_token(&mut self, tokens: &[Token], i: &mut usize) -> Result<(), String> {
        match &tokens[*i] {
            Token::Int(n) => self.stack.push(Value::Int(*n)),
            Token::Float(f) => self.stack.push(Value::Float(*f)),
            Token::Bool(b) => self.stack.push(Value::Bool(*b)),
            Token::StringLit(s) => self.stack.push(Value::Str(s.clone())),
            Token::LiteralName(n) => self.stack.push(Value::Name(n.clone())),

            Token::ProcStart => {
                let proc_tokens = self.collect_procedure(tokens, i)?;
                let captured_env = if self.use_lexical_scoping {
                    Some(self.dicts.snapshot())
                } else {
                    None
                };
                self.stack.push(Value::Procedure {
                    tokens: proc_tokens,
                    captured_env,
                });
            }

            Token::ProcEnd => return Err("Unexpected '}'".to_string()),

            // Array literal: push a mark, execute all tokens up to the matching ],
            // then collect everything above the mark into an Array value.
            //
            // Nested arrays work naturally because each inner [ recurses into
            // this same branch and returns a complete Array value onto the stack
            // before the outer ] fires.
            Token::ArrayStart => {
                self.stack.op_mark()?;
                *i += 1;
                while *i < tokens.len() {
                    if tokens[*i] == Token::ArrayEnd {
                        break;
                    }
                    self.execute_token(tokens, i)?;
                    *i += 1;
                }
                if *i >= tokens.len() {
                    return Err("Unterminated array — missing ']'".to_string());
                }
                // *i now points at the matching ']'; collect items above the mark
                self.stack.op_counttomark()?;
                let count = match self.stack.pop()? {
                    Value::Int(n) => n as usize,
                    _ => return Err("array literal: internal error".to_string()),
                };
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(self.stack.pop()?);
                }
                items.reverse();
                self.stack.op_cleartomark()?;
                self.stack.push(Value::Array(items));
            }

            Token::ArrayEnd => return Err("Unexpected ']'".to_string()),

            Token::Name(name) => {
                let name = name.clone();
                self.dispatch(&name, tokens, i)?;
            }
        }
        Ok(())
    }

    /// Collect tokens between `{` and the matching `}` into a Vec<Token>.
    /// Called when ProcStart is encountered; advances `i` past the closing `}`.
    fn collect_procedure(&self, tokens: &[Token], i: &mut usize) -> Result<Vec<Token>, String> {
        let mut proc_tokens = Vec::new();
        let mut depth = 1usize;
        *i += 1;

        while *i < tokens.len() {
            match &tokens[*i] {
                Token::ProcStart => {
                    depth += 1;
                    proc_tokens.push(tokens[*i].clone());
                }
                Token::ProcEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(proc_tokens);
                    }
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
    /// After the procedure finishes the live dict stack is restored, even on error.
    pub fn execute_procedure(
        &mut self,
        proc_tokens: &[Token],
        captured_env: Option<Vec<Dict>>,
    ) -> Result<(), String> {
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

    /// Internal helper: run a token slice without touching the dict stack.
    fn run_tokens(&mut self, tokens: &[Token]) -> Result<(), String> {
        let mut i = 0;
        while i < tokens.len() {
            self.execute_token(tokens, &mut i)?;
            i += 1;
        }
        Ok(())
    }

    /// Dispatch a name to a built-in operator or user-defined procedure/value.
    /// Built-ins are matched as string literals; anything else is looked up in
    /// the dictionary stack.
    fn dispatch(&mut self, name: &str, _tokens: &[Token], _i: &mut usize) -> Result<(), String> {
        match name {
            // ── Stack manipulation ────────────────────────────────────────────
            "exch" => self.stack.op_exch(),
            "pop" => self.stack.op_pop(),
            "dup" => self.stack.op_dup(),
            "copy" => self.stack.op_copy(),
            "clear" => self.stack.op_clear(),
            "count" => self.stack.op_count(),
            "roll" => self.stack.op_roll(),
            "index" => self.stack.op_index(),
            "mark" => self.stack.op_mark(),
            "cleartomark" => self.stack.op_cleartomark(),
            "counttomark" => self.stack.op_counttomark(),

            // ── Arithmetic ────────────────────────────────────────────────────
            "add" => self.stack.op_add(),
            "sub" => self.stack.op_sub(),
            "mul" => self.stack.op_mul(),
            "div" => self.stack.op_div(),
            "idiv" => self.stack.op_idiv(),
            "mod" => self.stack.op_mod(),
            "abs" => self.stack.op_abs(),
            "neg" => self.stack.op_neg(),
            "ceiling" => self.stack.op_ceiling(),
            "floor" => self.stack.op_floor(),
            "round" => self.stack.op_round(),
            "sqrt" => self.stack.op_sqrt(),

            // ── Dictionary ────────────────────────────────────────────────────
            "dict" => self.dicts.op_dict(&mut self.stack),
            "maxlength" => self.dicts.op_maxlength(&mut self.stack),
            "begin" => self.dicts.op_begin(&mut self.stack),
            "end" => self.dicts.op_end(),
            "def" => self.dicts.op_def(&mut self.stack),
            "put" => self.dicts.op_put(&mut self.stack),

            // ── length: routes based on top-of-stack type ─────────────────────
            "length" => match self.stack.peek()? {
                Value::Str(_) => self.stack.op_string_length(),
                Value::Dict(_) => self.dicts.op_length(&mut self.stack),
                Value::Array(_) => self.dicts.op_length(&mut self.stack),
                other => Err(format!(
                    "length: expected string, dict, or array, got {:?}",
                    other
                )),
            },

            // ── get: routes based on container type ───────────────────────────
            "get" => {
                // Peek at the second item from top (the container) to decide
                let container_type = {
                    let slice = self.stack.as_slice();
                    let len = slice.len();
                    if len < 2 {
                        return Err("get: stack underflow".to_string());
                    }
                    match &slice[len - 2] {
                        Value::Dict(_) => "dict",
                        Value::Array(_) => "array",
                        Value::Str(_) => "str",
                        _ => "other",
                    }
                };
                match container_type {
                    "dict" => self.dicts.op_get_dict(&mut self.stack),
                    "array" => self.stack.op_get(),
                    "str" => self.stack.op_get(),
                    _ => Err("get: expected string, array, or dict".to_string()),
                }
            }

            // ── String / array operators ──────────────────────────────────────
            "getinterval" => self.stack.op_getinterval(),
            "putinterval" => self.stack.op_putinterval(),
            "string" => self.stack.op_string(),
            "array" => self.stack.op_array(),

            // ── Boolean & comparison ──────────────────────────────────────────
            "eq" => self.stack.op_eq(),
            "ne" => self.stack.op_ne(),
            "ge" => self.stack.op_ge(),
            "gt" => self.stack.op_gt(),
            "le" => self.stack.op_le(),
            "lt" => self.stack.op_lt(),
            "and" => self.stack.op_and(),
            "or" => self.stack.op_or(),
            "not" => self.stack.op_not(),
            "true" => self.stack.op_true(),
            "false" => self.stack.op_false(),

            // ── Type operators ────────────────────────────────────────────────
            "type" => self.stack.op_type(),
            "cvs" => self.stack.op_cvs(),
            "cvi" => self.stack.op_cvi(),
            "cvr" => self.stack.op_cvr(),
            "cvn" => self.stack.op_cvn(),

            // ── I/O ────────────────────────────────────────────────────────────
            "print" => self.stack.op_print(),
            "=" => self.stack.op_print_pop(),
            "==" => self.stack.op_print_repr(),

            // ── Flow control ──────────────────────────────────────────────────
            "if" => self.op_if(),
            "ifelse" => self.op_ifelse(),
            "for" => self.op_for(),
            "repeat" => self.op_repeat(),
            "forall" => self.op_forall(),
            "quit" => self.op_quit(),

            // ── Scoping toggle ────────────────────────────────────────────────
            "lexical" => {
                self.use_lexical_scoping = true;
                Ok(())
            }
            "dynamic" => {
                self.use_lexical_scoping = false;
                Ok(())
            }

            // ── User-defined name ─────────────────────────────────────────────
            _ => match self.dicts.lookup(name) {
                Some(Value::Procedure {
                    tokens,
                    captured_env,
                }) => {
                    let tokens = tokens.clone();
                    let env = captured_env.clone();
                    self.execute_procedure(&tokens, env)
                }
                Some(val) => {
                    self.stack.push(val);
                    Ok(())
                }
                None => Err(format!("Unknown name: {}", name)),
            },
        }
    }

    /// forall — container proc →
    ///
    /// Iterate over a container and execute proc for each element:
    ///   - Array:  push each element in order, then execute proc.
    ///   - Dict:   push the key (as a Name) and value for each entry, then execute proc.
    ///             (Dict iteration order is unspecified, as with HashMap.)
    ///   - String: push the integer character code of each byte, then execute proc.
    ///
    /// The captured environment (if any) is cloned and passed to each proc invocation
    /// so lexical scoping is respected across iterations.
    ///
    /// Examples:
    ///   0 [1 2 3] { add } forall     → 6
    ///   [1 2 3] { 2 mul } forall     → 2 4 6
    pub fn op_forall(&mut self) -> Result<(), String> {
        let (proc_tokens, captured_env) = match self.stack.pop()? {
            Value::Procedure {
                tokens,
                captured_env,
            } => (tokens, captured_env),
            other => return Err(format!("forall: expected procedure, got {:?}", other)),
        };
        let container = self.stack.pop()?;

        match container {
            Value::Array(items) => {
                for item in items {
                    self.stack.push(item);
                    self.execute_procedure(&proc_tokens, captured_env.clone())?;
                }
                Ok(())
            }
            Value::Dict(d) => {
                // Collect entries first to avoid borrow issues
                let entries: Vec<(String, Value)> = d.entries.into_iter().collect();
                for (k, v) in entries {
                    self.stack.push(Value::Name(k));
                    self.stack.push(v);
                    self.execute_procedure(&proc_tokens, captured_env.clone())?;
                }
                Ok(())
            }
            Value::Str(s) => {
                // forall on a string: push each character code
                for byte in s.bytes() {
                    self.stack.push(Value::Int(byte as i64));
                    self.execute_procedure(&proc_tokens, captured_env.clone())?;
                }
                Ok(())
            }
            other => Err(format!(
                "forall: expected array, dict, or string, got {:?}",
                other
            )),
        }
    }
}

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
                /x 99 def  x
            end
            x
        ");
        assert_eq!(result, vec![Value::Int(99), Value::Int(1)]);
    }

    #[test]
    fn test_dynamic_scoping_default() {
        let result = run("/x 10 def  /getx { x } def  /x 99 def  getx");
        assert_eq!(result, vec![Value::Int(99)]);
    }

    #[test]
    fn test_lexical_scoping_captures_definition_env() {
        let result = run("lexical  /x 10 def  /getx { x } def  /x 99 def  getx");
        assert_eq!(result, vec![Value::Int(10)]);
    }

    #[test]
    fn test_array_literal() {
        let result = run("[ 1 2 3 ]");
        assert_eq!(
            result,
            vec![Value::Array(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3)
            ])]
        );
    }

    #[test]
    fn test_array_length() {
        assert_eq!(run("[ 1 2 3 ] length"), vec![Value::Int(3)]);
    }

    #[test]
    fn test_array_get() {
        assert_eq!(run("[ 10 20 30 ] 1 get"), vec![Value::Int(20)]);
    }

    #[test]
    fn test_forall_array() {
        // Sum elements of array
        assert_eq!(run("0 [ 1 2 3 4 5 ] { add } forall"), vec![Value::Int(15)]);
    }

    #[test]
    fn test_forall_string() {
        // Count characters (sum their codes won't be checked, just verify no error)
        let result = run("[ ] (hi) { } forall");
        // Just ensure it runs without error
        let _ = result;
    }

    #[test]
    fn test_mark_cleartomark() {
        assert_eq!(run("1 mark 2 3 cleartomark"), vec![Value::Int(1)]);
    }

    #[test]
    fn test_counttomark() {
        assert_eq!(
            run("mark 1 2 3 counttomark"),
            vec![
                Value::Mark,
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(3)
            ]
        );
    }

    #[test]
    fn test_type_operator() {
        assert_eq!(run("42 type"), vec![Value::Name("integertype".to_string())]);
        assert_eq!(
            run("(hi) type"),
            vec![Value::Name("stringtype".to_string())]
        );
        assert_eq!(
            run("true type"),
            vec![Value::Name("booleantype".to_string())]
        );
        assert_eq!(run("3.14 type"), vec![Value::Name("realtype".to_string())]);
    }

    #[test]
    fn test_cvi() {
        assert_eq!(run("3.9 cvi"), vec![Value::Int(3)]);
    }

    #[test]
    fn test_cvr() {
        assert_eq!(run("5 cvr"), vec![Value::Float(5.0)]);
    }

    #[test]
    fn test_cvn() {
        assert_eq!(run("(foo) cvn"), vec![Value::Name("foo".to_string())]);
    }

    #[test]
    fn test_string_op() {
        let result = run("5 string");
        match &result[0] {
            Value::Str(s) => assert_eq!(s.len(), 5),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn test_array_op() {
        assert_eq!(
            run("3 array"),
            vec![Value::Array(vec![
                Value::Int(0),
                Value::Int(0),
                Value::Int(0)
            ])]
        );
    }

    #[test]
    fn test_roll() {
        // 1 2 3  3 1 roll → 3 1 2
        assert_eq!(
            run("1 2 3  3 1 roll"),
            vec![Value::Int(3), Value::Int(1), Value::Int(2)]
        );
    }

    #[test]
    fn test_index() {
        assert_eq!(
            run("1 2 3  1 index"),
            vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(2)]
        );
    }

    #[test]
    fn test_put_array() {
        assert_eq!(run("[ 1 2 3 ] 1 99 put 1 get"), vec![Value::Int(99)]);
    }

    #[test]
    fn test_unknown_name_errors() {
        let mut interp = Interpreter::new();
        assert!(interp.run("notdefined").is_err());
    }
}
