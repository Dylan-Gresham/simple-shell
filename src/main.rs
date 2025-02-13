use shell::Shell;

pub mod shell;

fn main() {
    Shell::parse_args();

    println!("Hello, world!");
}
