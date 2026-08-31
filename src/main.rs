use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process;
use std::rc::Rc;
use std::str;

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
            process::exit(0);
        };

        // Eval
        let result = eval(&command, &mut buf_out);
        match result {
            Err(err) => write_to_buffer(&mut buf_out, err.to_string())?,
            Ok(Action::Exit) => process::exit(0),
            Ok(Action::Continue) => {}
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

/// Represents a command that can be executed.
#[derive(Clone)]
enum Command {
    Builtin {
        execute: Rc<dyn Fn(&[&str], &mut dyn Write) -> Result<Action>>,
    },
    Executable {
        path: PathBuf
    },
    Invalid
}

impl Command {
    fn builtin(execute: Rc<dyn Fn(&[&str], &mut dyn Write) -> Result<Action>>) -> Command {
        Command::Builtin { execute }
    }

    /// Returns an immutable smart pointer (reference-counted) to a function for this command.
    fn from_name(command_name: &str) -> Command {
        let mut builtin_by_name: HashMap<&str, Command> = HashMap::new();
        builtin_by_name.insert("echo", Command::builtin(Rc::new(execute_echo)));
        builtin_by_name.insert("exit", Command::builtin(Rc::new(execute_exit)));
        builtin_by_name.insert("type", Command::builtin(Rc::new(execute_type)));

        if let Some(builtin_command) = builtin_by_name.get(command_name) {
            return builtin_command.clone()
        }

        if let Some(executable_command) = check_executable_path(command_name) {
            return executable_command
        }

        Command::Invalid
    }

    /// Executes this command with the given arguments and output buffer. Returns the next action a REPL should take.
    fn execute(&self, args: &[&str], buf_out: &mut dyn Write) -> Result<Action> {
        match self {
            Command::Builtin { execute } => execute(&args, buf_out),
            Command::Executable { path } => {
                process::Command::new(path).args(&args[1..]).spawn()?.wait()?;
                Ok(Action::Continue)
            }
            Command::Invalid => {
                let &command_name = args.get(0).ok_or_else(|| anyhow!("expected an arg"))?;
                write_to_buffer(buf_out, format!("{command_name}: command not found\n"))?;
                Ok(Action::Continue)
            }
        }
    }
}

/// Returns an executable command if the program exists in PATH.
fn check_executable_path(command_name: &str) -> Option<Command> {
    // TODO: Consider handling raw paths (e.g. /usr/bin/python3).
    let path_dirs: Vec<PathBuf> = parse_path_env()
        .into_iter()
        .filter(|path_dir| path_dir.is_dir())
        .collect();

    for path_dir in path_dirs.iter() {
        let Ok(command_path) = path_dir.join(command_name).canonicalize() else { continue };
        if !command_path.is_file() { continue; }
        let Ok(metadata) = command_path.metadata() else { continue };
        // Executable iff at least one executable bit is set
        if metadata.permissions().mode() & 0o111 == 0 { continue };

        return Some(Command::Executable { path: command_path });
    }
    None
}

/// Parses the PATH environment variable into a list of directories.
fn parse_path_env() -> Vec<PathBuf> {
    match env::var_os("PATH") {
        Some(paths) => env::split_paths(&paths).collect(),
        None => Vec::new()
    }
}

/// Executes the given `raw_command`. Returns the action to perform next.
fn eval(raw_command: &str, buf_out: &mut impl Write) -> Result<Action> {
    let args: Vec<&str> = raw_command.split_whitespace().collect();
    let Some(&command_name) = args.get(0) else {
        return Ok(Action::Continue);
    };

    Command::from_name(command_name).execute(&args, buf_out)
}

/// Echoes the given arguments to stdout.
fn execute_echo(args: &[&str], buf_out: &mut dyn Write) -> Result<Action> {
    write_to_buffer(buf_out, format!("{}\n", args[1..].join(" ")))?;
    Ok(Action::Continue)
}

/// Exits the shell.
fn execute_exit(_args: &[&str], _buf_out: &mut dyn Write) -> Result<Action> {
    Ok(Action::Exit)
}

/// Outputs the type of each provided command (a shell builtin, executable, or not found).
fn execute_type(args: &[&str], buf_out: &mut dyn Write) -> Result<Action> {
    for &command_name in args[1..].iter() {
        let msg = match Command::from_name(command_name) {
            Command::Builtin { execute: _ } => format!("{command_name} is a shell builtin\n"),
            Command::Executable { path } => format!("{command_name} is {}\n", path.display()),
            Command::Invalid => format!("{command_name}: not found\n"),
        };
        write_to_buffer(buf_out, msg)?;
    }
    Ok(Action::Continue)
}

fn write_to_buffer(buf_out: &mut dyn Write, msg: impl AsRef<str>) -> Result<()> {
    buf_out.write(msg.as_ref().as_bytes())?;
    Ok(())
}

// TODO: Remove the need for serial tests.
#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::env;
    use std::fs::{File, Permissions};
    use tempfile;

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
    #[serial]
    fn eval_handles_executable() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let filepath = temp.path().join("my_executable");
        let file = File::create(&filepath)?;
        file.set_permissions(Permissions::from_mode(0o777))?;

        unsafe {
            env::set_var("PATH", temp.path());
        }

        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("my_executable", &mut buf_out)?;
        assert_eq!(str::from_utf8(&buf_out)?, "");
        assert_eq!(result, Action::Continue);
        Ok(())
    }

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

    #[test]
    fn eval_type_empty() -> Result<()> {
        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("type", &mut buf_out)?;
        assert_eq!(str::from_utf8(&buf_out)?, "");
        assert_eq!(result, Action::Continue);
        Ok(())
    }

    #[test]
    fn eval_type_builtins() -> Result<()> {
        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("type echo type", &mut buf_out)?;
        assert_eq!(
            str::from_utf8(&buf_out)?,
            "echo is a shell builtin\ntype is a shell builtin\n"
        );
        assert_eq!(result, Action::Continue);
        Ok(())
    }

    #[test]
    #[serial]
    fn eval_type_executable() -> Result<()> {
        let temp1 = tempfile::tempdir()?;
        let filepath1 = temp1.path().join("hello123");
        let file1 = File::create(&filepath1)?;
        file1.set_permissions(Permissions::from_mode(0o744))?;

        let temp2 = tempfile::tempdir()?;
        let filepath2 = temp2.path().join("world123");
        let file2 = File::create(&filepath2)?;
        file2.set_permissions(Permissions::from_mode(0o610))?;

        let filepath3 = temp2.path().join("not_executable");
        let file3 = File::create(&filepath3)?;
        file3.set_permissions(Permissions::from_mode(0o644))?;

        unsafe {
            env::set_var("PATH", env::join_paths([temp1.path(), temp2.path()])?);
        }

        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("type hello123 world123 not_executable", &mut buf_out)?;
        assert_eq!(
            str::from_utf8(&buf_out)?,
            format!(
                "hello123 is {}\nworld123 is {}\nnot_executable: not found\n",
                filepath1.canonicalize()?.display(),
                filepath2.canonicalize()?.display(),
            )
        );
        assert_eq!(result, Action::Continue);
        Ok(())
    }

    #[test]
    #[serial]
    fn eval_type_mixed() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let filepath = temp.path().join("my_executable");
        let file = File::create(&filepath)?;
        file.set_permissions(Permissions::from_mode(0o777))?;

        unsafe {
            env::set_var("PATH", temp.path());
        }

        let mut buf_out: Vec<u8> = Vec::new();
        let result = eval("type abacadabra my_executable exit 123456789", &mut buf_out)?;
        assert_eq!(
            str::from_utf8(&buf_out)?,
            format!(
                "abacadabra: not found\nmy_executable is {}\nexit is a shell builtin\n123456789: not found\n",
                filepath.canonicalize()?.display()
            )
        );
        assert_eq!(result, Action::Continue);
        Ok(())
    }
}
