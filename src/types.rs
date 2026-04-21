// types.rs — Shared runtime value type
//
// Defines the Value enum that every other module uses.
// Nothing else lives here — no operators, no stack logic.

use crate::lexer::Token;
use crate::dictionary::Dict;
use std::fmt;

/// Every value that can live on the PostScript operand stack.
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

    /// A literal name, e.g. /foo — stored as data, never executed
    Name(String),

    /// A procedure — a list of tokens to be executed later
    Procedure(Vec<Token>),

    /// A dictionary — created by `dict`, pushed/popped via `begin`/`end`
    Dict(Dict),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n)        => write!(f, "{}", n),
            Value::Float(n)      => write!(f, "{}", n),
            Value::Bool(b)       => write!(f, "{}", b),
            Value::Str(s)        => write!(f, "{}", s),
            Value::Name(n)       => write!(f, "{}", n),
            Value::Dict(d)       => write!(f, "-dict({}/{})-", d.entries.len(), d.capacity),
            Value::Procedure(tokens) => {
                write!(f, "{{ ")?;
                for t in tokens { write!(f, "{:?} ", t)?; }
                write!(f, "}}")
            }
        }
    }
}