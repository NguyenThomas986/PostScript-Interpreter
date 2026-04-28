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
// Scoping:
//   Under dynamic scoping (the default), procedures look up names in the live
//   dictionary stack at call time — whatever is on the stack when the procedure
//   runs is what it sees, regardless of where or when it was defined.
//
//   Under lexical scoping, when a { } block is parsed we take a snapshot of
//   the current dictionary stack and store it inside the resulting Procedure
//   value as `captured_env`. When the procedure is later called,
//   execute_procedure temporarily swaps the live stack out for this snapshot,
//   so all name lookups resolve against the definition-time environment instead
//   of the call-time environment. After the procedure body finishes — whether
//   successfully or with an error — the live stack is restored.
//
// Array literal [ ... ]:
//   On seeing ArrayStart (`[`), a Mark sentinel is pushed onto the operand
//   stack. All tokens up to the matching ArrayEnd (`]`) are executed normally,
//   so their computed values land on the stack above the mark. After the `]`,
//   counttomark tells us how many values to collect; they are popped into a
//   Vec<Value>, reversed to restore left-to-right order, and wrapped in a
//   Value::Array. Nested arrays work naturally because each inner `[` recurses
//   through this same branch and returns a fully formed Value::Array before the
//   outer `]` fires.

use crate::dictionary::{Dict, DictStack};
use crate::lexer::{Token, tokenize};
use crate::stack::OperandStack;
use crate::types::Value;

pub struct Interpreter {
    pub stack: OperandStack,
    pub dicts: DictStack,
    // Scoping mode flag. false = dynamic (default), true = lexical.
    // Toggled at runtime by the "lexical" and "dynamic" commands.
    // Checked every time a { } procedure body is parsed: if true, a snapshot
    // of the current dictionary stack is captured and stored inside the
    // resulting Value::Procedure. Procedures defined before the toggle retain
    // their original scoping behavior — this flag only affects future defs.
    pub use_lexical_scoping: bool,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
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
    /// like procedure bodies and array literals.
    ///
    /// This is the main dispatch point for every token produced by the lexer.
    /// Literal values are pushed directly onto the operand stack. Procedure
    /// bodies and array literals require consuming additional tokens from the
    /// slice before returning. Name tokens (without a leading slash) are
    /// routed to dispatch(), which either calls a built-in or looks the name
    /// up in the dictionary stack and executes whatever it finds there.
    pub fn execute_token(&mut self, tokens: &[Token], i: &mut usize) -> Result<(), String> {
        match &tokens[*i] {
            // Literal values: push directly onto the operand stack with no
            // computation. These are pure data — the stack is their only
            // destination.
            Token::Int(n) => self.stack.push(Value::Int(*n)),
            Token::Float(f) => self.stack.push(Value::Float(*f)),
            Token::Bool(b) => self.stack.push(Value::Bool(*b)),
            Token::StringLit(s) => self.stack.push(Value::Str(s.clone())),
            Token::LiteralName(n) => self.stack.push(Value::Name(n.clone())),

            // Procedure body: collect all tokens between `{` and the matching `}`
            // without executing them yet. They are stored as a token list inside
            // Value::Procedure and only executed when the procedure is called.
            //
            // The scoping decision happens here at definition time, not at call
            // time. If use_lexical_scoping is true, we snapshot the entire
            // dictionary stack right now and store it as `captured_env`. When
            // the procedure is later called, execute_procedure swaps that snapshot
            // in so name lookups see the definition-time bindings. If
            // use_lexical_scoping is false, captured_env is None, meaning the
            // procedure will use whatever the live dictionary stack is at the
            // moment it runs — classic dynamic scoping.
            //
            // This is why `lexical` must be written BEFORE defining any procedure
            // you want to behave with lexical scoping: the snapshot is taken here
            // when the `{` is parsed, not later when the procedure is invoked.
            Token::ProcStart => {
                let proc_tokens = self.collect_procedure(tokens, i)?;
                let captured_env = if self.use_lexical_scoping {
                    Some(self.dicts.snapshot()) // freeze the dict stack right now
                } else {
                    None // dynamic: no snapshot, resolve names at call time
                };
                self.stack.push(Value::Procedure {
                    tokens: proc_tokens,
                    captured_env,
                });
            }

            Token::ProcEnd => return Err("Unexpected '}'".to_string()),

            // Array literal: push a Mark sentinel, execute all tokens up to the
            // matching `]` so their values land on the stack above the mark, then
            // collect those values into a Vec and wrap them in Value::Array.
            //
            // The Mark acts as a boundary so we know exactly which values belong
            // to this array literal. counttomark counts how many values sit above
            // it; we pop that many, reverse them to restore left-to-right order
            // (the stack is LIFO, so the last-pushed element would otherwise end
            // up first), clear the mark, and push the finished array. Nesting
            // works automatically because each inner `[` recurses through this
            // same branch and finishes with a complete Value::Array on the stack
            // before the outer `]` is processed.
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
                // *i now points at the matching `]`; collect everything above the mark.
                self.stack.op_counttomark()?;
                let count = match self.stack.pop()? {
                    Value::Int(n) => n as usize,
                    _ => return Err("array literal: internal error".to_string()),
                };
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(self.stack.pop()?);
                }
                items.reverse(); // restore left-to-right order
                self.stack.op_cleartomark()?;
                self.stack.push(Value::Array(items));
            }

            Token::ArrayEnd => return Err("Unexpected ']'".to_string()),

            // A bare Name token (no leading `/`) means "execute this name".
            // We clone it immediately to release the borrow on `tokens` before
            // calling dispatch, which needs a mutable borrow of self.
            Token::Name(name) => {
                let name = name.clone();
                self.dispatch(&name, tokens, i)?;
            }
        }
        Ok(())
    }

    /// Collect the tokens between `{` and the matching `}` into a Vec<Token>.
    ///
    /// Called when ProcStart is encountered. Advances `i` past the closing `}`.
    /// The tokens are not executed here — they are stored raw and only run when
    /// the resulting procedure is later called. A depth counter handles nested
    /// braces: each inner `{` increments it and each `}` decrements it, so the
    /// collection only stops when the depth returns to zero, which corresponds
    /// to the closing brace that matches the opening `{` that triggered this call.
    fn collect_procedure(&self, tokens: &[Token], i: &mut usize) -> Result<Vec<Token>, String> {
        let mut proc_tokens = Vec::new();
        let mut depth = 1usize; // the opening `{` has already been consumed
        *i += 1;

        while *i < tokens.len() {
            match &tokens[*i] {
                Token::ProcStart => {
                    depth += 1; // entering a nested procedure body
                    proc_tokens.push(tokens[*i].clone());
                }
                Token::ProcEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(proc_tokens); // found the matching `}` — done
                    }
                    proc_tokens.push(tokens[*i].clone());
                }
                other => proc_tokens.push(other.clone()),
            }
            *i += 1;
        }
        Err("Unterminated procedure — missing '}'".to_string())
    }

    /// Execute a procedure's token list under the correct scoping environment.
    ///
    /// There are two cases depending on whether captured_env is present:
    ///
    /// 1. captured_env is Some(snapshot) — LEXICAL SCOPING
    ///    The snapshot was taken when the procedure was defined. We swap the
    ///    entire live dictionary stack out and replace it with this snapshot,
    ///    then run the procedure body. Every name lookup that happens during
    ///    execution will walk the snapshot instead of the live stack, so it
    ///    finds the definition-time bindings. After the body finishes — even
    ///    if it returned an error — we swap the live stack back so the caller's
    ///    environment is fully restored. The swap-restore pattern is what makes
    ///    lexical scoping work without needing Rc<RefCell<>> or any other
    ///    shared-mutation machinery.
    ///
    /// 2. captured_env is None — DYNAMIC SCOPING
    ///    We simply run the procedure tokens against the current live dictionary
    ///    stack without touching it. Name lookups will find whatever happens to
    ///    be in scope at the moment of the call, which may be different from
    ///    what was in scope when the procedure was defined.
    pub fn execute_procedure(
        &mut self,
        proc_tokens: &[Token],
        captured_env: Option<Vec<Dict>>,
    ) -> Result<(), String> {
        if let Some(env) = captured_env {
            // Swap in the definition-time snapshot.
            let live_stack = self.dicts.swap(env);
            // Run the body against the frozen environment.
            let result = self.run_tokens(proc_tokens);
            // Restore the live stack regardless of whether the body succeeded.
            self.dicts.swap(live_stack);
            result
        } else {
            // Dynamic scoping: run directly against the current live stack.
            self.run_tokens(proc_tokens)
        }
    }

    /// Internal helper: iterate over a token slice and execute each token in order.
    ///
    /// Does not touch the dictionary stack — that is handled by execute_procedure
    /// before this is called. Used both by execute_procedure (for procedure bodies)
    /// and by run (for top-level source strings).
    fn run_tokens(&mut self, tokens: &[Token]) -> Result<(), String> {
        let mut i = 0;
        while i < tokens.len() {
            self.execute_token(tokens, &mut i)?;
            i += 1;
        }
        Ok(())
    }

    /// Dispatch a name to a built-in operator or a user-defined procedure/value.
    ///
    /// Built-in operators are matched as string literals in the arms below. If
    /// the name does not match any built-in, execution falls through to the `_`
    /// arm, which walks the dictionary stack looking for the name. If the lookup
    /// finds a Procedure, we call execute_procedure on its token list (passing
    /// along any captured_env so lexical scoping is respected). If the lookup
    /// finds a plain value we push it onto the operand stack. If nothing is
    /// found, we return an "Unknown name" error.
    ///
    /// The `length` and `get` built-ins are polymorphic: they work on multiple
    /// container types. Both peek at the stack to decide which implementation to
    /// call rather than having a single monolithic handler.
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
            // `def` pops a value and a name and stores them in the top dictionary.
            // Which dictionary is "top" is controlled entirely by begin/end —
            // begin pushes a new dict, making it the target for subsequent defs,
            // and end pops it back off.
            "dict" => self.dicts.op_dict(&mut self.stack),
            "maxlength" => self.dicts.op_maxlength(&mut self.stack),
            "begin" => self.dicts.op_begin(&mut self.stack),
            "end" => self.dicts.op_end(),
            "def" => self.dicts.op_def(&mut self.stack),
            "put" => self.dicts.op_put(&mut self.stack),

            // ── length: polymorphic dispatch on top-of-stack type ─────────────
            // `length` is defined for strings, dicts, and arrays. We peek at the
            // top of the stack to determine which handler to call. Strings have
            // their own implementation in strings.rs; dicts and arrays share the
            // same op_length in dictionary.rs.
            "length" => match self.stack.peek()? {
                Value::Str(_) => self.stack.op_string_length(),
                Value::Dict(_) => self.dicts.op_length(&mut self.stack),
                Value::Array(_) => self.dicts.op_length(&mut self.stack),
                other => Err(format!(
                    "length: expected string, dict, or array, got {:?}",
                    other
                )),
            },

            // ── get: polymorphic dispatch on container type ───────────────────
            // `get` works on dicts, arrays, and strings. The container is the
            // second-from-top value (the key sits on top), so we inspect the
            // element at len-2 to decide which path to take. Dict lookups go to
            // dictionary.rs; array and string element access go to stack.rs.
            "get" => {
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
            // `lexical` flips the flag so that any { } block defined from this
            // point forward will have a snapshot of the current dict stack
            // embedded inside it at parse time. Procedures defined before this
            // toggle are not affected — they keep whatever captured_env they
            // already have. This is why `lexical` must appear before the
            // procedure definitions you want to behave lexically.
            "lexical" => {
                self.use_lexical_scoping = true;
                // Re-snapshot all existing procedures so they immediately
                // resolve names against the current environment, not the
                // live stack at call time.
                self.dicts.stamp_captured_envs();
                Ok(())
            }

            // `dynamic` does two things:
            //   1. Flips the flag so future { } blocks get no snapshot.
            //   2. Walks every dict on the stack and strips captured_env out
            //      of every procedure it finds.
            //
            // Step 2 is necessary because without it a procedure like foo that
            // was defined under lexical scoping would keep its snapshot and
            // keep running lexically even after `dynamic` is called. The TA
            // demo requires that `dynamic` followed by `20 foo` produces 40
            // (live stack lookup finds x=20), not 30 (snapshot lookup finds
            // x=10). Stripping captured_env from existing procedures makes
            // them fall back to the live stack on their next call, which is
            // exactly dynamic scoping behavior.
            "dynamic" => {
                self.use_lexical_scoping = false;
                // Walk every dictionary on the stack and clear captured_env
                // from every procedure value stored there.
                // Delegate to DictStack so we don't need to expose the
                // internal `stack` field outside of dictionary.rs.
                self.dicts.strip_captured_envs();
                Ok(())
            }

            // ── User-defined name fallthrough ─────────────────────────────────
            // Any name that did not match a built-in reaches here. We walk the
            // dictionary stack from top to bottom looking for a binding. Under
            // dynamic scoping this walk uses the live stack at call time. Under
            // lexical scoping, execute_procedure has already swapped the live
            // stack for the definition-time snapshot before dispatch is called,
            // so this same walk hits the captured bindings automatically — no
            // special case required here.
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
    /// Iterates over a container and executes proc for each element.
    ///
    ///   Array:  pushes each element in order, then executes proc.
    ///   Dict:   pushes the key (as a Name) followed by the value for each
    ///           entry, then executes proc. Iteration order is unspecified
    ///           because HashMap does not preserve insertion order.
    ///   String: pushes the integer byte value of each character, then
    ///           executes proc.
    ///
    /// forall lives here in interpreter.rs rather than dictionary.rs because it
    /// needs to call execute_procedure, which requires access to the full
    /// Interpreter struct. DictStack only has access to its own data.
    ///
    /// The captured_env is cloned on each iteration so every proc invocation
    /// gets the same frozen definition-time environment when lexical scoping is
    /// active. Under dynamic scoping captured_env is None, so the clone is a
    /// no-op and the live stack is used as usual.
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
                // Collect all entries into a Vec first to avoid holding a borrow
                // on `d` while also calling execute_procedure through self.
                let entries: Vec<(String, Value)> = d.entries.into_iter().collect();
                for (k, v) in entries {
                    self.stack.push(Value::Name(k));
                    self.stack.push(v);
                    self.execute_procedure(&proc_tokens, captured_env.clone())?;
                }
                Ok(())
            }
            Value::Str(s) => {
                // Push the integer byte value of each character in the string.
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
        // foo is defined when x=10, but x is rebound to 99 before calling foo.
        // Under dynamic scoping foo sees x=99 at call time → returns 99.
        let result = run("/x 10 def  /getx { x } def  /x 99 def  getx");
        assert_eq!(result, vec![Value::Int(99)]);
    }

    #[test]
    fn test_lexical_scoping_captures_definition_env() {
        // `lexical` must be set BEFORE defining the procedure so the snapshot
        // is taken at definition time (when x=10). When x is later rebound to
        // 99 and getx is called, it still sees x=10 from the captured snapshot.
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
        assert_eq!(run("0 [ 1 2 3 4 5 ] { add } forall"), vec![Value::Int(15)]);
    }

    #[test]
    fn test_forall_string() {
        let result = run("[ ] (hi) { } forall");
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
