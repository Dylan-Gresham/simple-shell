use std::ffi::CString;
use std::process::exit;

use libc::{
    c_char, c_int, pid_t, posix_spawn_file_actions_init, posix_spawn_file_actions_t,
    posix_spawnattr_init, posix_spawnattr_setflags, posix_spawnattr_setpgroup,
    posix_spawnattr_setsigdefault, posix_spawnattr_t, posix_spawnp, setpgid, sigemptyset, signal,
    sigset_t, tcsetpgrp, waitpid, POSIX_SPAWN_SETPGROUP, POSIX_SPAWN_SETSIGDEF, SIGINT, SIGQUIT,
    SIGTSTP, SIGTTIN, SIGTTOU, SIG_IGN, WIFEXITED,
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

    unsafe {
        let _ = signal(SIGINT, SIG_IGN);
        let _ = signal(SIGQUIT, SIG_IGN);
        let _ = signal(SIGTSTP, SIG_IGN);
        let _ = signal(SIGTTIN, SIG_IGN);
        let _ = signal(SIGTTOU, SIG_IGN);
    }

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
                            let mut ptrs: Vec<*mut c_char> =
                                cmd.iter().map(|s| s.as_ptr().cast_mut()).collect();
                            ptrs.push(std::ptr::null_mut());

                            let argv: *const *mut c_char = ptrs.as_ptr();
                            unsafe {
                                let child_pid: pid_t = pid_t::default();

                                // Setup spawn attributes
                                let mut attr: posix_spawnattr_t = std::mem::zeroed();
                                let mut file_actions: posix_spawn_file_actions_t =
                                    std::mem::zeroed();

                                posix_spawnattr_init(&mut attr);
                                posix_spawnattr_setflags(
                                    &mut attr,
                                    (POSIX_SPAWN_SETPGROUP | POSIX_SPAWN_SETSIGDEF)
                                        .try_into()
                                        .unwrap(),
                                );
                                posix_spawnattr_setpgroup(&mut attr, 0);

                                let mut sig_default: sigset_t = std::mem::zeroed();
                                sigemptyset(&mut sig_default);
                                posix_spawnattr_setsigdefault(&mut attr, &sig_default);

                                posix_spawn_file_actions_init(&mut file_actions);

                                let envp: Vec<CString> = std::env::vars()
                                    .map(|(key, value)| {
                                        CString::new(format!("{}={}", key, value)).unwrap()
                                    })
                                    .collect();

                                let mut env_ptrs: Vec<*mut c_char> = envp
                                    .into_iter()
                                    .map(|cstr| cstr.as_ptr() as *mut c_char)
                                    .collect();
                                env_ptrs.push(std::ptr::null_mut());

                                if posix_spawnp(
                                    child_pid as *mut pid_t,
                                    c,
                                    &file_actions,
                                    &attr,
                                    argv,
                                    env_ptrs.as_ptr(),
                                ) != 0
                                {
                                    eprintln!("`posix_spawn` failed");
                                    exit(1);
                                }

                                let _ = setpgid(child_pid, child_pid);
                                let _ = tcsetpgrp(shell.shell_terminal, child_pid);
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

    Ok(())
}
