mod app;
mod highlight;
mod ui;

use std::{io, path::PathBuf};

use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::CrosstermBackend,
    Terminal,
};

use app::App;
use ui::ui;

// Restores the terminal (leaves raw mode / the alternate screen) no matter
// how we exit: normal return, an early `?`, or a panic. Without this, any
// failure after EnterAlternateScreen leaves the terminal stuck showing a
// blank alternate screen forever, since the OS does not undo that on
// process exit -- it looks exactly like the app did nothing.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

fn main() -> io::Result<()> {
    let paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let mut app = App::new(paths)?;

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let _guard = TerminalGuard;

    // A panic inside run() unwinds past the guard's Drop only after this
    // hook runs, so restore the terminal here too before the default hook
    // prints the panic message -- otherwise the message is written into
    // the (about to vanish) alternate screen and never seen.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);
    terminal.show_cursor()?;

    result
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui(frame, app))?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => app.handle_key(key),
            Event::Mouse(mouse) => app.handle_mouse(mouse),
            _ => {}
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
