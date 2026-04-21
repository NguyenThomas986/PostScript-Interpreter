// main.rs — Entry point and REPL
//
// Only responsibility: read lines from stdin, hand them to the interpreter,
// and print the resulting stack. No operator logic lives here.

mod lexer;
mod types;
mod dictionary;
mod stack;
mod arithmetic;
mod boolean;
mod strings;
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
            Ok(0) => break,
            Ok(_) => {
                let input = line.trim();
                if input.is_empty() { continue; }
                if input == "quit" || input == "exit" { break; }

                match interp.run(input) {
                    Ok(_) => {
                        print!("stack: [");
                        for (idx, val) in interp.stack.as_slice().iter().enumerate() {
                            if idx > 0 { print!(", "); }
                            print!("{}", val);
                        }
                        println!("]");
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            Err(e) => { eprintln!("Error reading input: {}", e); break; }
        }
    }

    println!("Bye!");
}