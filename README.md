# PostScript Interpreter

<div align="center">

[![CI](https://github.com/nguyenthomas986/PostScript-Interpreter/actions/workflows/ci.yml/badge.svg)](https://github.com/nguyenthomas986/PostScript-Interpreter/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/nguyenthomas986/PostScript-Interpreter/graph/badge.svg)](https://codecov.io/gh/nguyenthomas986/PostScript-Interpreter)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

</div>

A PostScript interpreter implemented in Rust for CPTS 355: Programming Language Design.

## What is PostScript?

PostScript is a stack-based programming language created by [Adobe](https://www.adobe.com/products/postscript.html) that is used to describe how text and graphics should appear on a page.

Instead of just storing an image, a PostScript program gives instructions like:

- where to place text
- how to draw shapes
- how to format a page

A PostScript interpreter (like this project) reads those instructions and executes them step by step.

## Building and Running

### Prerequisites

Install Rust via [rustup.rs](https://rustup.rs):

**Windows (PowerShell)**
```powershell
winget install Rust.Rustup
```

**macOS (Terminal)**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installing, close and reopen your terminal, then verify:
```
rustc --version
cargo --version
```

---

### Build

```
cd PostScript-Interpreter
cargo build
```

For an optimized release build:
```
cargo build --release
```

---

### Run (Interactive REPL)

```
cargo run
```

This launches an interactive prompt where you can type PostScript commands one line at a time:

```
PostScript Interpreter
Type PostScript commands. Press Ctrl+C to exit.
----------------------------------------------
ps> 3 4 add
stack: [7]
ps> /x exch def
stack: []
ps> x
stack: [7]
ps> [ 1 2 3 ] { 2 mul } forall
stack: [2, 4, 6]
ps> quit
```

The current stack is printed after every line.

---

### Run Tests

```
cargo test
```

Unit tests live alongside their source files. Integration tests are in `tests/`.

To see a detailed coverage report locally:

```
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

---

## Scoping Behavior

PostScript uses **dynamic scoping** by default. This interpreter also supports **lexical (static) scoping**, toggled at runtime.

### Dynamic Scoping (default)

Procedures look up names in the dictionary stack **as it exists at call time**.

```postscript
/x 10 def
/getx { x } def
/x 99 def
getx        % returns 99 — sees the current x
```

### Lexical (Static) Scoping

When lexical scoping is enabled, a procedure captures its **definition-time environment**. Name lookups inside the procedure always resolve against the dictionary stack that existed when the procedure was defined.

```
lexical     % switch to lexical (static) scoping
dynamic     % switch back to dynamic scoping (default)
```

```postscript
lexical
/x 10 def
/getx { x } def
/x 99 def
getx        % returns 10 — captured x at definition time
```

---

## Supported Commands

### Stack Manipulation
| Command        | Description |
|----------------|-------------|
| `exch`         | Swap top two elements |
| `pop`          | Discard top element |
| `dup`          | Duplicate top element |
| `copy`         | Duplicate top n elements |
| `clear`        | Empty the stack |
| `count`        | Push number of elements on stack |
| `roll`         | Rotate top n elements j times |
| `index`        | Push a copy of the nth element from the top |
| `mark`         | Push a mark object onto the stack |
| `cleartomark`  | Pop everything down to and including the topmost mark |
| `counttomark`  | Push the number of elements above the topmost mark |

### Arithmetic
| Command   | Description |
|-----------|-------------|
| `add`     | Add top two numbers |
| `sub`     | Subtract top two numbers |
| `mul`     | Multiply top two numbers |
| `div`     | Divide (always float result) |
| `idiv`    | Integer division (truncates toward zero) |
| `mod`     | Modulo |
| `abs`     | Absolute value |
| `neg`     | Negate |
| `ceiling` | Round up to nearest integer |
| `floor`   | Round down to nearest integer |
| `round`   | Round to nearest integer |
| `sqrt`    | Square root (always float) |

### Dictionary
| Command      | Description |
|--------------|-------------|
| `dict`       | Create a dictionary with given capacity |
| `length`     | Number of entries in a dictionary, string, or array |
| `maxlength`  | Capacity of a dictionary |
| `begin`      | Push dictionary onto the dictionary stack |
| `end`        | Pop dictionary from the dictionary stack |
| `def`        | Bind name to value in the current dictionary |
| `put`        | Store a value at a key in a dict, or at an index in an array |
| `get`        | Retrieve a value by key from a dict, by index from an array or string |

### Strings
| Command         | Description |
|-----------------|-------------|
| `string`        | Allocate a zero-filled string of length n |
| `length`        | Length of a string (also works on dicts and arrays) |
| `get`           | Character code at index (also works on arrays and dicts) |
| `getinterval`   | Substring (also works on arrays) |
| `putinterval`   | Replace a substring in place |

### Arrays
| Command         | Description |
|-----------------|-------------|
| `array`         | Allocate an array of n zero values |
| `[ ... ]`       | Array literal — evaluate elements and collect into an array |
| `length`        | Number of elements (also works on strings and dicts) |
| `get`           | Element at index (also works on strings and dicts) |
| `put`           | Set element at index (also works on dicts) |
| `getinterval`   | Sub-array from index with given length |
| `forall`        | Execute a procedure for each element |

### Boolean and Bitwise
| Command | Description |
|---------|-------------|
| `eq`    | Equal |
| `ne`    | Not equal |
| `ge`    | Greater than or equal |
| `gt`    | Greater than |
| `le`    | Less than or equal |
| `lt`    | Less than |
| `and`   | Logical AND (booleans) or bitwise AND (integers) |
| `or`    | Logical OR (booleans) or bitwise OR (integers) |
| `not`   | Logical NOT (booleans) or bitwise NOT (integers) |
| `true`  | Push boolean true |
| `false` | Push boolean false |

### Type and Conversion
| Command | Description |
|---------|-------------|
| `type`  | Push a name representing the type of the top value (e.g. `integertype`, `stringtype`, `arraytype`) |
| `cvi`   | Convert to integer (truncates floats, parses strings) |
| `cvr`   | Convert to real/float (promotes ints, parses strings) |
| `cvs`   | Convert any value to its string representation |
| `cvn`   | Convert a string to a name |

### Flow Control
| Command  | Description |
|----------|-------------|
| `if`     | Execute procedure if condition is true |
| `ifelse` | Execute one of two procedures based on condition |
| `for`    | Loop with a counter: `init increment limit proc for` |
| `repeat` | Execute procedure n times |
| `forall` | Execute procedure for each element of an array, dict, or string |
| `quit`   | Terminate the interpreter |

### Input / Output
| Command | Description |
|---------|-------------|
| `print` | Write a string to stdout (no newline) |
| `=`     | Print top of stack as plain text, then pop |
| `==`    | Print top of stack in PostScript syntax, then pop |

---

## Type Names (returned by `type`)

| Value type  | Name returned    |
|-------------|------------------|
| Integer     | `integertype`    |
| Float/Real  | `realtype`       |
| Boolean     | `booleantype`    |
| String      | `stringtype`     |
| Name        | `nametype`       |
| Array       | `arraytype`      |
| Dictionary  | `dicttype`       |
| Procedure   | `proceduretype`  |
| Mark        | `marktype`       |

---

## Examples

### Array literals and forall
```postscript
[ 1 2 3 4 5 ] { 2 mul } forall
% stack: [2, 4, 6, 8, 10]

0 [ 1 2 3 4 5 ] { add } forall
% stack: [15]
```

### mark / cleartomark
```postscript
1 2 mark 3 4 cleartomark
% stack: [1, 2]

mark 10 20 30 counttomark
% stack: [-mark- 10 20 30 3]
```

### Type checking and conversion
```postscript
42 type =         % integertype
(hello) type =    % stringtype
3.9 cvi =         % 3
5 cvr =           % 5.0
(foo) cvn =       % /foo
```

### put and get on dicts
```postscript
5 dict
/name (Alice) put
/name get =       % Alice
```

### roll and index
```postscript
1 2 3  3 1 roll   % stack: [3 1 2]
1 2 3  1 index    % stack: [1 2 3 2]
```

---

## Project Structure

```
PostScript-Interpreter/
├── Cargo.toml
├── README.md
├── .gitignore
├── src/
│   ├── main.rs           # Entry point and REPL loop
│   ├── lib.rs            # Library root — re-exports all modules for integration tests
│   ├── lexer.rs          # Tokenizer — converts source text to Token stream
│   ├── types.rs          # Shared Value enum (Int, Float, Bool, Str, Name, Procedure, Dict, Array, Mark)
│   ├── stack.rs          # Operand stack + stack manipulation operators
│   ├── arithmetic.rs     # Arithmetic operators (add, sub, mul, div, etc.)
│   ├── boolean.rs        # Boolean, comparison, type, and conversion operators
│   ├── strings.rs        # String and array operators (get, getinterval, string, array, etc.)
│   ├── dictionary.rs     # Dictionary stack + dictionary operators (def, put, get, forall, etc.)
│   ├── control.rs        # Flow control operators (if, ifelse, for, repeat, quit)
│   ├── io_ops.rs         # I/O operators (print, =, ==)
│   └── interpreter.rs    # Core execution engine — token dispatch, array literals, forall, scoping
└── tests/
    └── integration_test.rs  # End-to-end tests covering full PostScript programs
```

---

## Author

Thomas Nguyen — CPTS 355, Spring 2026  
Instructor: Subu Kandaswamy
