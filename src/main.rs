use anyhow::Result;
use std::io::{self, BufRead, BufReader, Write};
use std::process::exit;
use std::str::{self};

/// Action for the REPL to perform.
#[derive(Debug, PartialEq)]
enum Action {
    Continue,
    Exit,
}

fn main() -> Result<()> {
    let mut buf_in = BufReader::new(io::stdin());
    let mut buf_out = io::stdout();
    loop {
        // Read
        print!("{}", create_prompt());
        io::stdout().flush()?;

        let Some(command) = read_line_from_buffer(&mut buf_in)? else {
            // Reached EOF; exit
            exit(0);
        };

        // Eval
        let result = eval(&command, &mut buf_out)?;
        match result {
            Action::Exit => exit(0),
            Action::Continue => {}
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

/// Executes the given `raw_command`. Returns the action to perform next.
fn eval(raw_command: &str, buf_out: &mut impl Write) -> Result<Action> {
    let args: Vec<&str> = raw_command.split_whitespace().collect();
    let Some(&command) = args.get(0) else {
        return Ok(Action::Continue);
    };

    match command {
        "echo" => {
            write_to_buffer(buf_out, format!("{}\n", args[1..].join(" ")))?;
            Ok(Action::Continue)
        }
        "exit" => Ok(Action::Exit),
        _ => {
            write_to_buffer(buf_out, format!("{command}: command not found\n"))?;
            Ok(Action::Continue)
        }
    }
}

fn write_to_buffer(buf_out: &mut impl Write, msg: impl AsRef<str>) -> Result<()> {
    buf_out.write(msg.as_ref().as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_prompt_works() {
        assert_eq!(create_prompt(), "$ ");
    }

    // ========== Read tests ==========

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

    // ========== Eval tests ==========

    #[test]
    fn eval_handles_invalid_command() -> Result<()> {
        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("abacadabra", &mut buf_out)?;
        assert_eq!(str::from_utf8(&buf_out)?, "abacadabra: command not found\n");
        assert_eq!(result, Action::Continue);
        Ok(())
    }

    #[test]
    fn eval_echo_empty() -> Result<()> {
        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("echo", &mut buf_out)?;
        assert_eq!(str::from_utf8(&buf_out)?, "\n");
        assert_eq!(result, Action::Continue);
        Ok(())
    }

    #[test]
    fn eval_echo_args() -> Result<()> {
        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("echo  hello     world  !  ", &mut buf_out)?;
        assert_eq!(str::from_utf8(&buf_out)?, "hello world !\n");
        assert_eq!(result, Action::Continue);
        Ok(())
    }

    #[test]
    fn eval_exit() -> Result<()> {
        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("exit", &mut buf_out)?;
        assert!(buf_out.is_empty());
        assert_eq!(result, Action::Exit);
        Ok(())
    }
}
