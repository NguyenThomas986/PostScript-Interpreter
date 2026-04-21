# PostScript Interpreter

A PostScript interpreter implemented in Rust for CPTS 355: Programming Language Design.

---

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

**Windows**
```powershell
cd PostScript-Interpreter
cargo build
```

**macOS**
```bash
cd PostScript-Interpreter
cargo build
```

For an optimized release build:
```
cargo build --release
```

---

### Run (Interactive REPL)

**Windows**
```powershell
cargo run
```

**macOS**
```bash
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
ps> quit
```

The current stack is printed after every line so you can see the interpreter's state.

---

### Run Tests

```
cargo test
```

All unit tests are located alongside the source files they test (`src/lexer.rs`, `src/interpreter.rs`) and integration tests are in `tests/`.

---

## Scoping Behavior

PostScript uses **dynamic scoping** by default, which is this interpreter's default mode. This interpreter also supports **lexical (static) scoping**, toggled via a runtime command.

### Dynamic Scoping (default)

When a procedure looks up a name, it searches the dictionary stack **as it exists at the time of the call** — not at the time the procedure was defined. This means a procedure can see variables defined after it was written, as long as they are on the dictionary stack when it runs.

**Example:**
```postscript
/x 10 def
/getx { x } def

/x 99 def   % redefine x
getx        % returns 99, not 10 — sees the current x
```

### Lexical (Static) Scoping

When lexical scoping is enabled, a procedure captures its **definition-time environment**. Name lookups inside the procedure resolve against the dictionary stack that existed when the procedure was defined, not when it is called.

**Toggling lexical scoping:**

At the `ps>` prompt, type:
```
lexical     % switch to lexical (static) scoping
dynamic     % switch back to dynamic scoping (default)
```

**Same example under lexical scoping:**
```postscript
lexical
/x 10 def
/getx { x } def

/x 99 def   % redefine x
getx        % returns 10 — captured x at definition time
```

This demonstrates the core difference: dynamic scoping follows the **call-time** environment, lexical scoping follows the **definition-time** environment.

---

## Supported Commands

### Stack Manipulation
| Command | Description |
|---------|-------------|
| `exch`  | Swap top two elements |
| `pop`   | Discard top element |
| `dup`   | Duplicate top element |
| `copy`  | Duplicate top n elements |
| `clear` | Empty the stack |
| `count` | Push number of elements on stack |

### Arithmetic
| Command   | Description |
|-----------|-------------|
| `add`     | Add top two numbers |
| `sub`     | Subtract top two numbers |
| `mul`     | Multiply top two numbers |
| `div`     | Divide (float result) |
| `idiv`    | Integer division |
| `mod`     | Modulo |
| `abs`     | Absolute value |
| `neg`     | Negate |
| `ceiling` | Round up to nearest integer |
| `floor`   | Round down to nearest integer |
| `round`   | Round to nearest integer |
| `sqrt`    | Square root |

### Dictionary
| Command      | Description |
|--------------|-------------|
| `dict`       | Create dictionary with given capacity |
| `length`     | Number of entries in dictionary or string |
| `maxlength`  | Capacity of dictionary |
| `begin`      | Push dictionary onto dictionary stack |
| `end`        | Pop dictionary from dictionary stack |
| `def`        | Bind name to value in current dictionary |

### Strings
| Command         | Description |
|-----------------|-------------|
| `length`        | Length of string |
| `get`           | Character at index |
| `getinterval`   | Substring |
| `putinterval`   | Replace substring |

### Boolean and Bitwise
| Command | Description |
|---------|-------------|
| `eq`    | Equal |
| `ne`    | Not equal |
| `ge`    | Greater than or equal |
| `gt`    | Greater than |
| `le`    | Less than or equal |
| `lt`    | Less than |
| `and`   | Logical or bitwise AND |
| `or`    | Logical or bitwise OR |
| `not`   | Logical or bitwise NOT |
| `true`  | Push true |
| `false` | Push false |

### Flow Control
| Command  | Description |
|----------|-------------|
| `if`     | Execute procedure if condition is true |
| `ifelse` | Execute one of two procedures based on condition |
| `for`    | Loop with counter |
| `repeat` | Execute procedure n times |
| `quit`   | Terminate the interpreter |

### Input / Output
| Command | Description |
|---------|-------------|
| `print` | Write string to stdout |
| `=`     | Print top of stack as text, pop it |
| `==`    | Print top of stack in PostScript representation, pop it |

---

## Unimplemented Commands

The following commands from the PostScript specification were not implemented due to genuine constraints of the Rust type system or being outside the required subset for this assignment.

| Command | Reason |
|---------|--------|
| `mark` / `cleartomark` / `counttomark` | Outside the required command subset (Appendix A). These require a special mark value on the stack; not needed for any required command. |
| `type` | Outside the required subset. Returning a PostScript name object representing a runtime type requires additional infrastructure not needed elsewhere. |
| `cvs` / `cvi` / `cvr` / `cvn` | Type conversion operators are outside the required subset. Basic int/float coercion is handled implicitly in arithmetic. |
| `forall` | Outside the required subset. Iteration over dictionary entries requires exposing dictionary internals in a way not needed by any other required command. |
| `string` | String allocation operator is outside the required subset. String literals via `( )` syntax cover all required use cases. |
| `put` (on dictionaries) | The `put` operator for dictionaries is outside the required subset. Dictionary mutation is fully covered by `def`. |

---

## Project Structure

```
PostScript-Interpreter/
├── Cargo.toml
├── README.md
├── .gitignore
└── src/
    ├── main.rs           # Entry point and REPL loop
    ├── lexer.rs          # Tokenizer — converts source text to Token stream
    └── interpreter.rs    # Execution engine — operand stack, dictionary stack, all operators
```

---

## Author

Thomas Nguyen — CPTS 355, Spring 2026
Instructor: Subu Kandaswamy
