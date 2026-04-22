// types.rs — Shared runtime value type

use crate::lexer::Token;
use crate::dictionary::Dict;
use std::fmt;

/// Every value that can live on the PostScript operand stack.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    /// A literal name, e.g. /foo — stored as data, never executed
    Name(String),
    /// A procedure — a list of tokens to be executed later.
    Procedure {
        tokens: Vec<Token>,
        captured_env: Option<Vec<Dict>>,
    },
    /// A dictionary — created by `dict`, pushed/popped via `begin`/`end`
    Dict(Dict),
    /// A mark — placed by `mark`, consumed by `cleartomark` / `counttomark`
    Mark,
    /// An array — created by `[ ... ]` or `array`
    Array(Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n)        => write!(f, "{}", n),
            Value::Float(n)      => write!(f, "{}", n),
            Value::Bool(b)       => write!(f, "{}", b),
            Value::Str(s)        => write!(f, "{}", s),
            Value::Name(n)       => write!(f, "/{}", n),
            Value::Mark          => write!(f, "-mark-"),
            Value::Dict(d)       => write!(f, "-dict({}/{})-", d.entries.len(), d.capacity),
            Value::Array(items)  => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Procedure { tokens, .. } => {
                write!(f, "{{ ")?;
                for t in tokens { write!(f, "{:?} ", t)?; }
                write!(f, "}}")
            }
        }
    }
}