use libc::{
    c_char, chdir, getpid, getpwuid, getuid, isatty, kill, pid_t, setpgid, signal, tcsetattr,
    tcsetpgrp, termios, SIGINT, SIGQUIT, SIGTERM, SIGTSTP, SIGTTIN, SIGTTOU, SIG_DFL, SIG_IGN,
    STDIN_FILENO, TCSADRAIN,
};
use std::env;
use std::ffi::CString;
use std::process::exit;

pub struct Shell {
    pub shell_is_interactive: bool,
    pub shell_pgid: pid_t,
    pub shell_tmodes: termios,
    pub shell_terminal: i32,
    pub prompt: String,
}

impl Shell {
    /// Initialize the shell for use. Allocate all datastructures, grab control of the terminal and
    /// put the shell in its own process group.
    ///
    /// NOTE: This function will block until the shell is in its own program group. Attaching a
    /// debugger will always cause this function to fail because the debugger maintains control of
    /// he subprocess it is debugging.
    pub fn init() -> Self {
        let shell_terminal = STDIN_FILENO;
        let shell_is_interactive = unsafe { isatty(shell_terminal) } == 1;
        let shell_pgid: pid_t = unsafe { getpid() };
        let shell_tmodes: termios = unsafe { std::mem::zeroed() };
        let prompt = Shell::get_prompt(String::from("MY_PROMPT"));

        unsafe {
            setpgid(shell_pgid, shell_pgid);
            tcsetpgrp(shell_terminal, shell_pgid);

            let _ = signal(SIGINT, SIG_IGN);
            let _ = signal(SIGQUIT, SIG_IGN);
            let _ = signal(SIGTSTP, SIG_IGN);
            let _ = signal(SIGTTIN, SIG_IGN);
            let _ = signal(SIGTTOU, SIG_IGN);
        }

        Self {
            shell_is_interactive,
            shell_pgid,
            shell_tmodes,
            shell_terminal,
            prompt,
        }
    }

    pub fn destroy(&self) {
        unsafe {
            tcsetpgrp(self.shell_terminal, self.shell_pgid);
            tcsetattr(self.shell_terminal, TCSADRAIN, &self.shell_tmodes);

            signal(SIGINT, SIG_DFL);
            signal(SIGQUIT, SIG_DFL);
            signal(SIGTSTP, SIG_DFL);
            signal(SIGTTIN, SIG_DFL);
            signal(SIGTTOU, SIG_DFL);

            kill(getpid(), SIGTERM);
        }
    }

    /// Set the shell prompt. This function will attempt to load a prompt from the requested
    /// environment variable, if the environment variable is not set, a default prompt of "shell>"
    /// is returned.
    ///
    /// ## Parameter(s)
    ///
    /// - `env: String` The environment variable
    ///
    /// ## Return(s)
    ///
    /// The prompt from the environment variable or the default prompt.
    pub fn get_prompt(env: String) -> String {
        match env::var(env) {
            Ok(prompt) => prompt,
            Err(_) => String::from("shell>"),
        }
    }

    /// Changes the current working directory of the shell. Uses the Linux system call `chdir`.
    /// With no arguments, the users home directory is used as the directory to change to.
    ///
    /// ## Returns
    ///
    /// - `Ok(())` if the directory was successfully changed.
    /// - `Err(isize)` if the directory failed to change.
    pub fn change_dir(dir: Vec<CString>) -> Result<(), isize> {
        // If we weren' passsed a directory to go to, use libc to navigate to the
        // user's home directory
        if dir.len() <= 1 {
            match env::var("HOME") {
                // If the HOME environment variable is set, use it
                Ok(home_dir) => {
                    let cstring = CString::new(home_dir).unwrap();
                    return match unsafe { chdir(cstring.as_ptr() as *const c_char) } {
                        0 => Ok(()),
                        other => Err(other.try_into().unwrap()),
                    };
                }
                // If it's not set, get it from the UID
                Err(_) => unsafe {
                    let uid = getuid();
                    let passwd = getpwuid(uid);

                    return match chdir((*passwd).pw_dir as *const c_char) {
                        0 => Ok(()),
                        other => Err(other.try_into().unwrap()),
                    };
                },
            }
        }

        match unsafe { chdir(dir.get(1).unwrap().as_ptr() as *const c_char) } {
            0 => Ok(()),
            other => Err(other.try_into().unwrap()),
        }
    }

    /// Convert line read from the user into format that will work with `execvp`. We limit the
    /// number of arguments to `ARG_MAX` loaded from sysconf.
    ///
    /// ## Parameter(s)
    ///
    /// - `line: String` The line to process.
    ///
    /// ## Returns
    ///
    /// - `Ok(Vec<*mut c_char>)` if the line was parsed without issue.
    /// - `Err(String)` if there was an issue parsing the line.
    pub fn cmd_parse(line: String) -> Result<Vec<CString>, String> {
        // Parse the line into a vector of CStrings
        Ok(line
            .trim()
            .split(" ")
            .map(|s| CString::new(s).unwrap())
            .collect())
    }

    /// Trim the whitespace from the start and end of a string. For example "   ls -a   " becomes
    /// "ls -a". This function modifies the argument `line` so that all printable chars are moved
    /// to the front of the string.
    ///
    /// ## Parameter(s)
    ///
    /// - `line: &mut String` A reference to the `String` to trim.
    pub fn trim_white(line: String) -> String {
        line.trim().to_string()
    }

    /// Takes an argument list and checks if the first argument is a built in command such as exit,
    /// cd, jobs, etc. If the command is a built in command this function will handle the command.
    /// If the first argument is *NOT* a built in command, this function will exit immediately.
    ///
    /// ## Parameter(s)
    ///
    /// - `argv: Vec<String>` The argument list to check
    ///
    ///
    /// ## Returns
    ///
    /// - `Ok(())` if this function handled the command as a built in.
    /// - `Err(isize)` if the command wasn't a built in and was *NOT* handled or the command failed
    ///   to execute and returned a non-zero status code..
    pub fn do_builtin(&self, argv: Vec<CString>) -> Result<(), isize> {
        if argv.is_empty() {
            Err(0)
        } else {
            let c_cstr = argv.first().unwrap();
            let builtin_cmd = c_cstr.to_str().unwrap();
            if builtin_cmd == "exit" {
                if argv.len() > 1 {
                    exit(
                        argv.get(1)
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .parse::<i32>()
                            .unwrap(),
                    );
                }
                exit(0);
            } else if builtin_cmd == "cd" {
                Shell::change_dir(argv)
            } else if builtin_cmd == "history" {
                let mut history_file_contents: String =
                    std::fs::read_to_string("history.txt").unwrap_or_default();

                // Remove the leading "#V2" from the history file if it's not the default
                if !history_file_contents.is_empty() {
                    history_file_contents = history_file_contents
                        [history_file_contents.char_indices().nth(4).unwrap().0..]
                        .to_string();
                }

                println!("{}", history_file_contents);
                Ok(())
            } else {
                Err(-1)
            }
        }
    }

    /// Parse command line args from the user when the shell was launched.
    pub fn parse_args() {
        let mut args = std::env::args();
        if args.len() > 1 {
            let arg = args.nth(1).unwrap();
            if arg == "-v" {
                println!(
                    "Simple Shell v{}.{} written by Dylan Gresham",
                    env!("CARGO_PKG_VERSION_MAJOR"),
                    env!("CARGO_PKG_VERSION_MINOR")
                );

                exit(0);
            } else if arg == "-h" {
                println!("Usage: simple-shell [-v | -h]\n");
                println!("\t-v\t\tPrints the major and minor version of this program.");
                println!("\t-h\t\tPrints this usage message.");

                exit(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_parse_two() {
        // The string we want to parse from the user
        // foo -v
        let stng: String = String::from("foo -v");

        let actual = Shell::cmd_parse(stng).unwrap();

        let expected: Vec<CString> =
            vec![CString::new("foo").unwrap(), CString::new("-v").unwrap()];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_cmd_parse() {
        let rval: Vec<CString> = Shell::cmd_parse(String::from("ls -a -l")).unwrap();

        let expected: Vec<CString> = vec![
            CString::new("ls").unwrap(),
            CString::new("-a").unwrap(),
            CString::new("-l").unwrap(),
        ];

        assert_eq!(expected, rval);
    }

    #[test]
    fn test_trim_white_no_whitespace() {
        let rval = Shell::trim_white(String::from("ls -a"));

        assert_eq!("ls -a", rval);
    }

    #[test]
    fn test_trim_white_start_whitespace() {
        let rval = Shell::trim_white(String::from("  ls -a"));

        assert_eq!("ls -a", rval);
    }

    #[test]
    fn test_trim_white_end_whitespace() {
        let rval = Shell::trim_white(String::from("ls -a  "));

        assert_eq!("ls -a", rval);
    }

    #[test]
    fn test_trim_white_both_whitespace() {
        let rval = Shell::trim_white(String::from(" ls -a "));

        assert_eq!("ls -a", rval);
    }

    #[test]
    fn test_trim_white_all_whitespace() {
        let rval = Shell::trim_white(String::from(" "));

        assert_eq!("", rval);
    }

    #[test]
    fn test_get_prompt_default() {
        if env::var("MY_PROMPT").is_ok() {
            env::remove_var("MY_PROMPT");
        }

        let prompt = Shell::get_prompt(String::from("MY_PROMPT"));

        assert_eq!("shell>", prompt);
    }

    #[test]
    fn test_get_prompt_custom() {
        env::set_var("MY_PROMPT", "foo>");

        let prompt = Shell::get_prompt(String::from("MY_PROMPT"));

        assert_eq!("foo>", prompt);
    }

    #[test]
    fn test_ch_dir_home() {
        let cmd = Shell::cmd_parse(String::from("cd")).unwrap();
        let expected = env::var("HOME").unwrap();
        let _ = Shell::change_dir(cmd).unwrap();

        let actual = env::current_dir().unwrap().to_str().unwrap().to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_ch_dir_root() {
        let cmd = Shell::cmd_parse(String::from("cd /")).unwrap();
        let expected = String::from("/");
        let _ = Shell::change_dir(cmd).unwrap();

        let actual = env::current_dir().unwrap().to_str().unwrap().to_string();

        assert_eq!(expected, actual);
    }
}
