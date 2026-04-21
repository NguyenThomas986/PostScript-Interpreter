// main.rs — Entry point for the PostScript interpreter
//
// This file does two things:
//   1. Declares the modules that make up the interpreter (lexer, interpreter)
//   2. Runs a REPL (Read-Eval-Print Loop) so the user can type PostScript
//      commands interactively, one line at a time.
//
// Later steps will wire the REPL into the real lexer and interpreter.
// For now it just echoes input so we can confirm the project structure compiles.
 
mod lexer;
mod interpreter;
 
use std::io::{self, BufRead, Write};
 
fn main() {
    println!("PostScript Interpreter");
    println!("Type PostScript commands. Press Ctrl+C to exit.");
    println!("----------------------------------------------");
 
    let stdin = io::stdin();
    let stdout = io::stdout();
 
    loop {
        // Print a prompt so the user knows we're waiting for input
        print!("ps> ");
        stdout.lock().flush().expect("Failed to flush stdout");
 
        // Read one line from stdin
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,                  // EOF (Ctrl+D on Unix, Ctrl+Z on Windows)
            Ok(_) => {
                let input = line.trim();
                if input == "quit" || input == "exit" {
                    break;
                }
                // TODO (Step 2+): pass `input` to the lexer, then interpreter
                // For now, just echo it back so we can confirm the loop works
                println!("  echo: {}", input);
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
 
    println!("Bye!");
}