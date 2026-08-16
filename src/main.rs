use std::io::{self, Write};

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    let mut command = String::new();
    io::stdin().read_line(&mut command).unwrap();
    command = command.trim_end().to_string();
    println!("{command}: command not found");
}
