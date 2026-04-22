// lexer.rs — PostScript Tokenizer
//
// Responsibility: take a raw &str of PostScript source and break it into
// a Vec<Token> that the interpreter can process one token at a time.
//
// The lexer does NOT evaluate anything — it only classifies characters into
// typed tokens. All execution logic lives in interpreter.rs.

// ── Token type ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Int(i64),
    Float(f64),
    Bool(bool),
    StringLit(String),
    LiteralName(String),
    Name(String),
    ProcStart,
    ProcEnd,
    ArrayStart,
    ArrayEnd,
}

// ── Public API ───────────────────────────────────────────────────────────────

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let n = chars.len();

    while i < n {
        let ch = chars[i];

        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        if ch == '%' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if ch == '{' {
            tokens.push(Token::ProcStart);
            i += 1;
            continue;
        }
        if ch == '}' {
            tokens.push(Token::ProcEnd);
            i += 1;
            continue;
        }

        if ch == '[' {
            tokens.push(Token::ArrayStart);
            i += 1;
            continue;
        }
        if ch == ']' {
            tokens.push(Token::ArrayEnd);
            i += 1;
            continue;
        }

        if ch == '(' {
            i += 1;
            let (s, new_i) = read_string(&chars, i, n)?;
            tokens.push(Token::StringLit(s));
            i = new_i;
            continue;
        }

        if ch == '/' {
            i += 1;
            let (word, new_i) = read_word(&chars, i, n);
            if word.is_empty() {
                return Err("Bare '/' with no name following it".to_string());
            }
            tokens.push(Token::LiteralName(word));
            i = new_i;
            continue;
        }

        let (word, new_i) = read_word(&chars, i, n);
        if !word.is_empty() {
            tokens.push(classify_word(&word));
            i = new_i;
            continue;
        }

        i += 1;
    }

    Ok(tokens)
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn read_word(chars: &[char], start: usize, n: usize) -> (String, usize) {
    let delimiters =
        |c: char| c.is_whitespace() || matches!(c, '(' | ')' | '{' | '}' | '[' | ']' | '%');
    let mut end = start;
    while end < n && !delimiters(chars[end]) {
        end += 1;
    }
    (chars[start..end].iter().collect(), end)
}

fn read_string(chars: &[char], start: usize, n: usize) -> Result<(String, usize), String> {
    let mut s = String::new();
    let mut i = start;
    let mut depth: usize = 1;

    while i < n {
        let c = chars[i];
        match c {
            '(' => {
                depth += 1;
                s.push('(');
                i += 1;
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    i += 1;
                    return Ok((s, i));
                } else {
                    s.push(')');
                    i += 1;
                }
            }
            '\\' => {
                i += 1;
                if i < n {
                    let escaped = match chars[i] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '(' => '(',
                        ')' => ')',
                        other => other,
                    };
                    s.push(escaped);
                    i += 1;
                }
            }
            other => {
                s.push(other);
                i += 1;
            }
        }
    }

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
    // 0. Boolean literals FIRST (avoid parsing confusion)
    if word == "true" {
        return Token::Bool(true);
    }
    if word == "false" {
        return Token::Bool(false);
    }

    // 1. Float detection FIRST if it looks like a float
    // (fixes 0.0, 1.0, etc. being misclassified as Int)
    if word.contains('.')
        && let Ok(f) = word.parse::<f64>()
    {
        return Token::Float(f);
    }

    // 2. Integer
    if let Ok(n) = word.parse::<i64>() {
        return Token::Int(n);
    }

    // 3. Float fallback (handles cases like scientific notation)
    if let Ok(f) = word.parse::<f64>() {
        return Token::Float(f);
    }

    // 4. Name
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
        assert_eq!(
            tokens,
            vec![
                Token::Name("add".to_string()),
                Token::Name("exch".to_string()),
                Token::Name("def".to_string()),
            ]
        );
    }

    #[test]
    fn test_proc_delimiters() {
        let tokens = tokenize("{ add }").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::ProcStart,
                Token::Name("add".to_string()),
                Token::ProcEnd,
            ]
        );
    }

    #[test]
    fn test_array_delimiters() {
        let tokens = tokenize("[ 1 2 3 ]").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::ArrayStart,
                Token::Int(1),
                Token::Int(2),
                Token::Int(3),
                Token::ArrayEnd,
            ]
        );
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
