use anyhow::Result;
use std::io::{self, Write};

/// Reads the user's input.
fn read() -> Result<String> {
    print!("$ ");
    io::stdout().flush()?;

    let mut command = String::new();
    io::stdin().read_line(&mut command)?;
    Ok(command.trim_end().to_string())
}

/// Executes the given `command`.
fn eval(command: &str) -> Result<()> {
    anyhow::bail!("{command}: command not found")
}

fn main() -> Result<()> {
    loop {
        let command = read()?;
        match eval(&command) {
            Err(msg) => println!("{msg}"),
            Ok(()) => todo!(),
        };
    }
}
