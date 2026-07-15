mod shell;
mod executor;
mod history;
mod jobs;
mod script;
mod prompt;
mod utils;

use anyhow;

use crate::shell::Shell;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // `vsh -c "command"`: run one command non-interactively and exit with
    // its status, instead of starting the interactive prompt loop. Used
    // both directly and by command substitution ($()/backticks), which
    // re-invokes this binary this way to capture output.
    if args.get(1).map(String::as_str) == Some("-c") {
        let command = args.get(2).cloned().unwrap_or_default();
        let code = Shell::new().run_single(&command)?;
        std::process::exit(code);
    }

    Shell::new().run()?;
    Ok(())
}
