mod shell;
mod executor;
mod history;
mod script;
mod prompt;
mod utils;

use anyhow;

use crate::shell::Shell;

fn main() -> anyhow::Result<()> {
    Shell::new().run()?;
    Ok(())
}
