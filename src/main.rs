// main.rs — Entry point for the PostScript interpreter
//
// Declares modules and runs a REPL that feeds user input into the interpreter.

mod lexer;
mod interpreter;

use std::io::{self, BufRead, Write};
use interpreter::Interpreter;

fn main() {
    println!("PostScript Interpreter");
    println!("Type PostScript commands. Press Ctrl+C to exit.");
    println!("----------------------------------------------");

    let mut interp = Interpreter::new();
    let stdin = io::stdin();
    let stdout = io::stdout();

    loop {
        print!("ps> ");
        stdout.lock().flush().expect("Failed to flush stdout");

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = line.trim();
                if input.is_empty() { continue; }
                if input == "quit" || input == "exit" { break; }

                // Run the input through the interpreter
                match interp.run(input) {
                    Ok(_) => {
                        // Print the current stack so the user can see what happened
                        print!("stack: [");
                        for (idx, val) in interp.operand_stack.iter().enumerate() {
                            if idx > 0 { print!(", "); }
                            print!("{}", val);
                        }
                        println!("]");
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }

    println!("Bye!");
}