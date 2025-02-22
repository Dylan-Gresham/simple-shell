use libc::{c_char, execvp};
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
    let builtin_cmds = vec!["ls", "cd", "exit"];

    loop {
        let readline = rl.readline(&shell.prompt);
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                match Shell::cmd_parse(line) {
                    Ok(cmd) => {
                        if builtin_cmds.contains(&cmd.get(0).unwrap().to_str().unwrap()) {
                            let _ = shell.do_builtin(cmd);
                        } else {
                            let c_cstr = cmd.get(0).unwrap();
                            let c = c_cstr.as_ptr() as *const c_char;
                            let mut ptrs: Vec<*const c_char> =
                                cmd.iter().map(|s| s.as_ptr()).collect();
                            ptrs.push(std::ptr::null());

                            let argv: *const *const c_char = ptrs.as_ptr();
                            unsafe {
                                let code = execvp(c, argv);

                                if code != 0 {
                                    eprintln!("Error code: {code}");
                                }
                            }
                        }
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
