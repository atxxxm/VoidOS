use std::{
    fs,
    io::{self, Read, Write},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let content = if let Some(path) = args.first() {
        match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("less: cannot open '{path}': {e}");
                std::process::exit(1);
            }
        }
    } else {
        let mut input = String::new();
        if io::stdin().read_to_string(&mut input).is_err() {
            return;
        }
        input
    };

    let lines: Vec<&str> = content.lines().collect();
    if let Err(e) = run_pager(&lines) {
        eprintln!("less: {e}");
        std::process::exit(1);
    }
}

// Restores the terminal no matter how run_pager exits -- an early `?` or a
// panic would otherwise leave it stuck in raw mode / the alternate screen,
// same lesson learned the hard way with bit's editor.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn run_pager(lines: &[&str]) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let mut top = 0usize;

    loop {
        let (_, rows) = terminal::size()?;
        let page_size = (rows as usize).saturating_sub(1).max(1);

        draw(lines, top, page_size)?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char(' ') | KeyCode::PageDown => {
                    top = (top + page_size).min(lines.len().saturating_sub(1));
                }
                KeyCode::Char('b') | KeyCode::PageUp => {
                    top = top.saturating_sub(page_size);
                }
                KeyCode::Down | KeyCode::Enter => {
                    top = (top + 1).min(lines.len().saturating_sub(1));
                }
                KeyCode::Up => top = top.saturating_sub(1),
                KeyCode::Char('g') => top = 0,
                KeyCode::Char('G') => top = lines.len().saturating_sub(page_size),
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(lines: &[&str], top: usize, page_size: usize) -> io::Result<()> {
    let mut out = io::stdout();
    execute!(out, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    let end = (top + page_size).min(lines.len());
    for line in &lines[top..end] {
        write!(out, "{line}\r\n")?;
    }

    let pct = if lines.is_empty() {
        100
    } else {
        (end * 100 / lines.len()).min(100)
    };
    write!(
        out,
        "-- {pct}% ({end}/{}) -- q: quit, space: page down, b: page up",
        lines.len()
    )?;
    out.flush()
}
