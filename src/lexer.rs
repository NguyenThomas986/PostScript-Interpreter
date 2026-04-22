// lexer.rs — PostScript Tokenizer
//
// Responsibility: take a raw &str of PostScript source and break it into
// a Vec<Token> that the interpreter can process one token at a time.
//
// The lexer does NOT evaluate anything — it only classifies characters into
// typed tokens. All execution logic lives in interpreter.rs.

// ── Token type ───────────────────────────────────────────────────────────────

/// Every distinct kind of value or syntax element in PostScript source code.
///
/// Examples:
///   42          → Token::Int(42)
///   3.14        → Token::Float(3.14)
///   true        → Token::Bool(true)
///   (hello)     → Token::StringLit("hello".to_string())
///   /foo        → Token::LiteralName("foo".to_string())
///   add         → Token::Name("add".to_string())
///   {           → Token::ProcStart
///   }           → Token::ProcEnd
///   [           → Token::ArrayStart
///   ]           → Token::ArrayEnd
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A whole number, e.g. 42 or -7
    Int(i64),

    /// A floating-point number, e.g. 3.14 or -2.5
    Float(f64),

    /// The literals `true` or `false`
    Bool(bool),

    /// A parenthesized string, e.g. (hello world) → "hello world"
    StringLit(String),

    /// A name preceded by `/`, e.g. /foo — pushed as data, never executed
    LiteralName(String),

    /// A bare word that will be looked up in the dictionary and executed,
    /// e.g. add, def, exch, myvar
    Name(String),

    /// The `{` character — marks the beginning of a procedure body
    ProcStart,

    /// The `}` character — marks the end of a procedure body
    ProcEnd,

    /// The `[` character — marks the beginning of an array literal
    ArrayStart,

    /// The `]` character — marks the end of an array literal
    ArrayEnd,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Tokenize a PostScript source string into a flat list of Tokens.
///
/// The function scans left-to-right, consuming one token per iteration.
/// Whitespace and `%` comments are skipped silently.
///
/// # Errors
/// Returns an `Err(String)` with a message if the input is malformed
/// (e.g. an unterminated string literal).
pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let n = chars.len();

    while i < n {
        let ch = chars[i];

        // ── Whitespace: skip ─────────────────────────────────────────────
        if ch.is_whitespace() { i += 1; continue; }

        // ── Comments: % … end-of-line — skip the whole line ─────────────
        if ch == '%' {
            while i < n && chars[i] != '\n' { i += 1; }
            continue;
        }

        // ── Procedure delimiters ─────────────────────────────────────────
        if ch == '{' { tokens.push(Token::ProcStart);  i += 1; continue; }
        if ch == '}' { tokens.push(Token::ProcEnd);    i += 1; continue; }

        // ── Array literal delimiters ─────────────────────────────────────
        // [ and ] are tokenized here; the interpreter handles collecting
        // the evaluated elements into a Value::Array at execution time.
        if ch == '[' { tokens.push(Token::ArrayStart); i += 1; continue; }
        if ch == ']' { tokens.push(Token::ArrayEnd);   i += 1; continue; }

        // ── PostScript string literal  ( ... ) ───────────────────────────
        // Strings may contain nested balanced parentheses.
        // Backslash escape sequences are also handled.
        if ch == '(' {
            i += 1; // skip opening '('
            let (s, new_i) = read_string(&chars, i, n)?;
            tokens.push(Token::StringLit(s));
            i = new_i;
            continue;
        }

        // ── Literal name  /foo ───────────────────────────────────────────
        // The slash means "push this name as a value, do not execute it".
        if ch == '/' {
            i += 1; // skip the '/'
            let (word, new_i) = read_word(&chars, i, n);
            if word.is_empty() {
                return Err("Bare '/' with no name following it".to_string());
            }
            tokens.push(Token::LiteralName(word));
            i = new_i;
            continue;
        }

        // ── Numbers and operator names ───────────────────────────────────
        // Collect a contiguous run of non-delimiter characters, then decide
        // what kind of token it is.
        let (word, new_i) = read_word(&chars, i, n);
        if !word.is_empty() {
            tokens.push(classify_word(&word));
            i = new_i;
            continue;
        }

        // ── Anything unrecognised: skip (should not normally happen) ─────
        i += 1;
    }

    Ok(tokens)
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Read characters until a delimiter is hit, return the word and the new index.
///
/// Delimiters are: whitespace, `(`, `)`, `{`, `}`, `[`, `]`, `%`
fn read_word(chars: &[char], start: usize, n: usize) -> (String, usize) {
    let delimiters = |c: char| c.is_whitespace() || matches!(c, '(' | ')' | '{' | '}' | '[' | ']' | '%');
    let mut end = start;
    while end < n && !delimiters(chars[end]) {
        end += 1;
    }
    (chars[start..end].iter().collect(), end)
}

/// Read the inside of a PostScript string starting just after the opening `(`.
///
/// Handles:
///   - Nested parentheses  (a (b) c)  → "a (b) c"
///   - Backslash escapes   \n \t \\ \( \)
///
/// Returns (string_contents, index_after_closing_paren).
fn read_string(chars: &[char], start: usize, n: usize) -> Result<(String, usize), String> {
    let mut s = String::new();
    let mut i = start;
    // depth starts at 1 because we already consumed the opening '('
    let mut depth: usize = 1;

    while i < n {
        let c = chars[i];
        match c {
            // Nested open paren — include it in the string, increase depth
            '(' => { depth += 1; s.push('('); i += 1; }
            // Close paren — either ends the string or closes a nested level
            ')' => {
                depth -= 1;
                if depth == 0 {
                    i += 1; // consume the ')'
                    return Ok((s, i));
                } else {
                    s.push(')');
                    i += 1;
                }
            }
            // Backslash escape sequences
            '\\' => {
                i += 1; // skip the backslash
                if i < n {
                    let escaped = match chars[i] {
                        'n'  => '\n', 't'  => '\t', 'r'  => '\r',
                        '\\' => '\\', '('  => '(', ')'  => ')',
                        other => other, // unknown escape: keep the char as-is
                    };
                    s.push(escaped);
                    i += 1;
                }
            }
            // Ordinary character
            other => { s.push(other); i += 1; }
        }
    }

    // If we exit the loop without depth reaching 0, the string was never closed
    Err("Unterminated string literal — missing closing ')'".to_string())
}

/// Decide what kind of token a bare word is.
///
/// Priority:
///   1. Integer  (try parsing as i64)
///   2. Float    (try parsing as f64)
///   3. Bool     ("true" / "false")
///   4. Name     (everything else — operator or user-defined name)
fn classify_word(word: &str) -> Token {
    // 1. Try integer first (avoids misclassifying "-3" as a float)
    if let Ok(n) = word.parse::<i64>() { return Token::Int(n); }
    // 2. Try float
    if let Ok(f) = word.parse::<f64>() { return Token::Float(f); }
    // 3. Boolean literals
    if word == "true"  { return Token::Bool(true); }
    if word == "false" { return Token::Bool(false); }
    // 4. Everything else is a name (operator or user variable)
    Token::Name(word.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integers() {
        let tokens = tokenize("42 -7 0").unwrap();
        assert_eq!(tokens, vec![Token::Int(42), Token::Int(-7), Token::Int(0)]);
    }

    #[test]
    fn test_floats() {
        let tokens = tokenize("3.14 -2.5").unwrap();
        assert_eq!(tokens, vec![Token::Float(3.14), Token::Float(-2.5)]);
    }

    #[test]
    fn test_booleans() {
        let tokens = tokenize("true false").unwrap();
        assert_eq!(tokens, vec![Token::Bool(true), Token::Bool(false)]);
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize("(hello world)").unwrap();
        assert_eq!(tokens, vec![Token::StringLit("hello world".to_string())]);
    }

    #[test]
    fn test_nested_string() {
        let tokens = tokenize("(a (b) c)").unwrap();
        assert_eq!(tokens, vec![Token::StringLit("a (b) c".to_string())]);
    }

    #[test]
    fn test_literal_name() {
        let tokens = tokenize("/foo").unwrap();
        assert_eq!(tokens, vec![Token::LiteralName("foo".to_string())]);
    }

    #[test]
    fn test_operator_name() {
        let tokens = tokenize("add exch def").unwrap();
        assert_eq!(tokens, vec![
            Token::Name("add".to_string()),
            Token::Name("exch".to_string()),
            Token::Name("def".to_string()),
        ]);
    }

    #[test]
    fn test_proc_delimiters() {
        let tokens = tokenize("{ add }").unwrap();
        assert_eq!(tokens, vec![
            Token::ProcStart,
            Token::Name("add".to_string()),
            Token::ProcEnd,
        ]);
    }

    #[test]
    fn test_array_delimiters() {
        let tokens = tokenize("[ 1 2 3 ]").unwrap();
        assert_eq!(tokens, vec![
            Token::ArrayStart,
            Token::Int(1), Token::Int(2), Token::Int(3),
            Token::ArrayEnd,
        ]);
    }

    #[test]
    fn test_comment_skipped() {
        let tokens = tokenize("42 % this is a comment\n 7").unwrap();
        assert_eq!(tokens, vec![Token::Int(42), Token::Int(7)]);
    }

    #[test]
    fn test_unterminated_string_error() {
        assert!(tokenize("(oops").is_err());
    }
}