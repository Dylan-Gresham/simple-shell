use libc::{chdir, geteuid, getpwnam, pid_t};
use std::ffi::{CStr, CString};
use std::str::FromStr;
use std::{env, mem};
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
    pub fn get_prompt(&mut self, env: String) {
        match env::var(env) {
            Ok(prompt) => self.prompt = prompt,
            Err(_) => self.prompt = String::from("shell>"),
        };
    }

    /// Changes the current working directory of the shell. Uses the Linux system call `chdir`.
    /// With no arguments, the users home directory is used as the directory to change to.
    ///
    /// ## Returns
    ///
    /// - `Ok(())` if the directory was successfully changed.
    /// - `Err(isize)` if the directory failed to change.
    pub fn change_dir(dir: CString) -> Result<(), isize> {
        // If we weren' passsed a directory to go to, use libc to navigate to the
        // user's home directory
        if dir.is_empty() {
            // Get uid of the person who launched the process
            let uid: u32 = unsafe { geteuid() };

            // From the uid, get the username
            let username = unsafe {
                let mut result = std::ptr::null_mut();
                let amt = match libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) {
                    n if n < 0 => 512 as usize,
                    n => n as usize,
                };
                let mut buf = Vec::with_capacity(amt);
                let mut passwd: libc::passwd = mem::zeroed();

                match libc::getpwuid_r(
                    uid,
                    &mut passwd,
                    buf.as_mut_ptr(),
                    buf.capacity() as libc::size_t,
                    &mut result,
                ) {
                    0 if !result.is_null() => {
                        let ptr = passwd.pw_name as *const _;
                        let username: &CStr = CStr::from_ptr(ptr);
                        CString::from(username)
                    }
                    _ => CString::from_str("root").unwrap(),
                }
            };

            // Using the username, get the home directory
            let passwd_struct: *mut libc::passwd =
                unsafe { getpwnam(username.as_ptr() as *const i8) };
            let home_dir = unsafe { (*passwd_struct).pw_dir };
            return match unsafe { chdir(home_dir) } {
                0 => Ok(()),
                other => Err(other.try_into().unwrap()),
            };
        }

        match unsafe { chdir(dir.as_ptr() as *const i8) } {
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
    /// - `Ok(String)` if the line was parsed without issue.
    /// - `Err(String)` if there was an issue parsing the line.
    pub fn cmd_parse(line: String) -> Result<String, String> {
        let _trimmed = line.trim().to_string();
        todo!()
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
    pub fn do_builtin(&self, _argv: Vec<String>) -> Result<(), ()> {
        todo!()
    }

    /// Parse command line args from the user when the shell was launched.
    pub fn parse_args() {
        todo!()
    }
}
