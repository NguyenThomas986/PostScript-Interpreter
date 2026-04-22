use postscript_interpreter::interpreter::Interpreter;
use std::io::{self, BufRead, Write};

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
                if input.is_empty() {
                    continue;
                }

                match interp.run(input) {
                    Ok(_) => {
                        print!("stack: [");
                        for (idx, val) in interp.stack.as_slice().iter().enumerate() {
                            if idx > 0 {
                                print!(", ");
                            }
                            print!("{}", val);
                        }
                        println!("]");
                    }
                    Err(e) if e == "__quit__" => break,
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
