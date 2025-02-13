use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use shell::Shell;

pub mod shell;

fn main() -> Result<()> {
    Shell::parse_args();

    let mut rl = DefaultEditor::new()?;
    if rl.load_history("history.txt").is_err() {
        eprintln!("No previous history.");
    }

    let shell: Shell = Shell::init();

    loop {
        let readline = rl.readline(&shell.prompt);
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                match Shell::cmd_parse(line) {
                    Ok(cmd) => {
                        let _ = shell.do_builtin(cmd);
                    }
                    Err(err) => eprintln!("Error parsing command: {:?}", err),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    let _ = rl.save_history("history.txt");

    Ok(())
}
