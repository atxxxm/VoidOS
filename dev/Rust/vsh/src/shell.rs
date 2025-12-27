// Main functionality of the shell

use anyhow;

pub struct Shell;

impl Shell {
    pub fn new() -> Self {
        // set PATH
        unsafe { std::env::set_var("PATH", "/bin:/sbin"); }

        Self
    }

    // Run a command
    fn run_cmd(&self, command: &str, is_script: bool) -> anyhow::Result<()> {
        todo!()
    }

    // Main loop vsh
    pub fn run(&self) -> anyhow::Result<()> {
        todo!()
    }
}