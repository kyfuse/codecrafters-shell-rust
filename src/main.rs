use anyhow::{Result, bail};
use std::io::{self, BufRead, BufReader, Write};
use std::process::exit;

/// Action for the REPL to perform.
#[derive(Debug, PartialEq)]
enum Action {
    Exit,
}

fn main() -> Result<()> {
    let mut stdin = BufReader::new(io::stdin());
    loop {
        // Read
        print!("{}", create_prompt());
        io::stdout().flush()?;

        let Some(command) = read_line_from_buffer(&mut stdin)? else {
            // Reached EOF; exit
            exit(0);
        };

        // Eval
        let result = eval(&command);

        // Print
        match result {
            Err(msg) => println!("{msg}"),
            Ok(Action::Exit) => exit(0),
        };
    }
}

/// Creates a terminal prompt.
fn create_prompt() -> String {
    "$ ".to_string()
}

/// Trims the end of the line being read.
fn read_line_from_buffer(buf: &mut impl BufRead) -> Result<Option<String>> {
    let mut line = String::new();
    let bytes_read = buf.read_line(&mut line)?;
    match bytes_read {
        0 => Ok(None),
        _ => Ok(Some(line.trim_end().to_string())),
    }
}

/// Executes the given `command`. Returns the action to perform next.
fn eval(command: &str) -> Result<Action> {
    match command {
        "exit" => Ok(Action::Exit),
        _ => bail!("{command}: command not found"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_prompt_works() {
        assert_eq!(create_prompt(), "$ ");
    }

    #[test]
    fn read_line_from_buffer_strips_trailing_whitespace() -> Result<()> {
        let mut buf = BufReader::new(" hi !  \n".as_bytes());
        assert_eq!(read_line_from_buffer(&mut buf)?, Some(" hi !".to_string()));
        Ok(())
    }

    #[test]
    fn read_line_from_buffer_handles_multiple_reads_and_eof() -> Result<()> {
        let mut buf = BufReader::new("hello\n world\n\n".as_bytes());
        assert_eq!(read_line_from_buffer(&mut buf)?, Some("hello".to_string()));
        assert_eq!(read_line_from_buffer(&mut buf)?, Some(" world".to_string()));
        assert_eq!(read_line_from_buffer(&mut buf)?, Some("".to_string()));
        assert_eq!(read_line_from_buffer(&mut buf)?, None);
        Ok(())
    }

    #[test]
    fn eval_handles_invalid_command() {
        let result = eval("abacadabra");
        result.expect_err("expected error on invalid command");
    }

    #[test]
    fn eval_exit() -> Result<()> {
        let result = eval("exit")?;
        assert_eq!(result, Action::Exit);
        Ok(())
    }
}
