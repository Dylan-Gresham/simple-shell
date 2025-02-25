use std::process::exit;

use libc::{
    abort, c_char, c_int, execvp, fork, getpid, pid_t, setpgid, signal, tcsetpgrp, waitpid, SIGINT,
    SIGQUIT, SIGTSTP, SIGTTIN, SIGTTOU, SIG_DFL,
};
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

    let builtin_cmds = ["cd", "exit", "history"];

    loop {
        let readline = rl.readline(&shell.prompt);
        match readline {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line.as_str());
                match Shell::cmd_parse(line) {
                    Ok(cmd) => {
                        let c_cstr = cmd.first().unwrap();
                        let first_cmd = c_cstr.to_str().unwrap();
                        if builtin_cmds.contains(&first_cmd) {
                            if first_cmd == "exit" || first_cmd == "history" {
                                let _ = rl.save_history("history.txt");
                            }

                            let _ = shell.do_builtin(cmd);
                        } else {
                            let c = c_cstr.as_ptr() as *const c_char;
                            let mut ptrs: Vec<*const c_char> =
                                cmd.iter().map(|s| s.as_ptr()).collect();
                            ptrs.push(std::ptr::null());

                            let argv: *const *const c_char = ptrs.as_ptr();
                            unsafe {
                                let fork_pid: pid_t = fork();

                                if fork_pid == 0 {
                                    // Successfully spawned a new process, give control to child
                                    let child_pid: pid_t = getpid();
                                    setpgid(child_pid, child_pid);
                                    tcsetpgrp(shell.shell_terminal, child_pid);

                                    // Set signals
                                    signal(SIGINT, SIG_DFL);
                                    signal(SIGQUIT, SIG_DFL);
                                    signal(SIGTSTP, SIG_DFL);
                                    signal(SIGTTIN, SIG_DFL);
                                    signal(SIGTTOU, SIG_DFL);

                                    // Tell it to execute the non-builtin command
                                    execvp(c, argv);
                                    exit(1);
                                } else if fork_pid < 0 {
                                    eprintln!("Failed to fork a new process.");
                                    abort();
                                }

                                setpgid(fork_pid, fork_pid);
                                tcsetpgrp(shell.shell_terminal, fork_pid);

                                let status: c_int = c_int::default();
                                let wait = waitpid(fork_pid, status as *mut c_int, 0);
                                if wait == -1 {
                                    eprintln!("waidpid failed with -1 code");
                                }

                                tcsetpgrp(shell.shell_terminal, shell.shell_pgid);
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

        println!();
    }

    let _ = rl.save_history("history.txt");
    shell.destroy();

    Ok(())
}
