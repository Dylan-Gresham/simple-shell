use libc::{c_char, chdir, execvp, pid_t};
use std::env;
use std::ffi::CString;
use termios::os::linux::termios;

pub struct Shell {
    pub shell_is_interactive: bool,
    pub shell_pgid: pid_t,
    pub shell_tmodes: termios,
    pub shell_terminal: isize,
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
        todo!()
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
    pub fn change_dir(dir: Vec<*mut c_char>) -> Result<(), isize> {
        // If we weren' passsed a directory to go to, use libc to navigate to the
        // user's home directory
        if dir.len() <= 1 {
            let home_dir: CString = CString::new(env::var("HOME").unwrap()).unwrap();
            return match unsafe { chdir(home_dir.as_ptr() as *const i8) } {
                0 => Ok(()),
                other => Err(other.try_into().unwrap()),
            };
        }

        match unsafe { chdir(*dir.get(1).unwrap()) } {
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
    pub fn cmd_parse(line: String) -> Result<Vec<*mut c_char>, String> {
        // Parse the line into a vector of CStrings
        let argv: Vec<CString> = line
            .trim()
            .to_string()
            .split(" ")
            .map(|s| CString::new(s).unwrap())
            .collect();

        // Convert from CStrings to a vector of c_char to be compatible with execvp.
        // Can't do this all in one operation since the CString's would be deallocated after the
        // closure and invalidate the borrow as a ptr and cast.
        let mut argv: Vec<*mut c_char> = argv.iter().map(|s| s.as_ptr() as *mut c_char).collect();

        // Null terminate the vector
        argv.push(std::ptr::null_mut());

        Ok(argv)
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
    /// - `Err(())` if the command wasn't a built in and was *NOT* handled.
    pub fn do_builtin(&self, argv: Vec<CString>) -> Result<(), ()> {
        if argv.len() < 1 {
            Err(())
        } else {
            let c = argv.get(0).unwrap().as_ptr() as *const c_char;
            let mut ptrs: Vec<*const c_char> = argv.iter().map(|s| s.as_ptr()).collect();
            ptrs.push(std::ptr::null());

            let argv: *const *const c_char = ptrs.as_ptr();
            unsafe {
                execvp(c, argv);
            }

            Ok(())
        }
    }

    /// Parse command line args from the user when the shell was launched.
    pub fn parse_args() {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;

    use libc::{c_char, getcwd, size_t};

    use super::*;

    #[test]
    fn test_cmd_parse_two() {
        // The string we want to parse from the user
        // foo -v
        let stng: String = String::from("foo -v");

        let actual = Shell::cmd_parse(stng);

        let expected: Result<Vec<*mut i8>, String> =
            Ok(vec!["foo".as_ptr() as *mut i8, "-v".as_ptr() as *mut i8]);

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_cmd_parse() {
        let rval = Shell::cmd_parse(String::from("ls -a -l")).unwrap();

        assert_eq!("ls".as_ptr() as *mut i8, *(rval.get(0).unwrap()));
        assert_eq!("-a".as_ptr() as *mut i8, *(rval.get(1).unwrap()));
        assert_eq!("-l".as_ptr() as *mut i8, *(rval.get(2).unwrap()));
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

        let size: size_t = env::var("PATH_MAX").unwrap().parse::<size_t>().unwrap();
        let buffer = String::with_capacity(size);
        let actual = unsafe { getcwd(buffer.as_ptr() as *mut i8, size) };

        unsafe {
            let c_str = CStr::from_ptr(actual as *const c_char);
            let rust_string = c_str.to_string_lossy().into_owned();

            assert_eq!(expected, rust_string);

            // Reclaim ownership so it gets dropped after
            let _ = CString::from_raw(actual as *mut c_char);
        }
    }

    #[test]
    fn test_ch_dir_root() {
        let cmd = Shell::cmd_parse(String::from("cd /")).unwrap();
        let expected = String::from("/");
        let _ = Shell::change_dir(cmd).unwrap();

        let size: size_t = env::var("PATH_MAX").unwrap().parse::<size_t>().unwrap();
        let buffer = String::with_capacity(size);
        let actual = unsafe { getcwd(buffer.as_ptr() as *mut i8, size) };

        unsafe {
            let c_str = CStr::from_ptr(actual as *const c_char);
            let rust_string = c_str.to_string_lossy().into_owned();

            assert_eq!(expected, rust_string);

            // Reclaim ownership so it gets dropped after
            let _ = CString::from_raw(actual as *mut c_char);
        }
    }
}
