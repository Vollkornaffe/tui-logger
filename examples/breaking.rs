use std::{io, sync::mpsc, thread, time};

use log::*;
use ratatui::prelude::*;
use std::env;
use tui_logger::*;

use self::crossterm_backend::*;
#[cfg(not(feature = "crossterm"))]
compile_error!("this example only works with 'crossterm'");

struct App {
    state: TuiWidgetState,
}

fn main() -> anyhow::Result<()> {
    init_logger(LevelFilter::Trace)?;
    set_default_level(LevelFilter::Trace);

    let mut dir = env::temp_dir();
    dir.push("tui-logger_demo.log");
    let file_options = TuiLoggerFile::new(dir.to_str().unwrap())
        .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        .output_file(false)
        .output_separator(':');
    set_log_file(file_options);
    debug!(target:"App", "Logging to {}", dir.to_str().unwrap());
    debug!(target:"App", "Logging initialized");

    let mut terminal = init_terminal()?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    App::new().start(&mut terminal)?;

    restore_terminal()?;
    terminal.clear()?;

    Ok(())
}

impl App {
    pub fn new() -> App {
        let state = TuiWidgetState::new().set_default_display_level(LevelFilter::Info);

        App { state }
    }

    pub fn start(mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        // Use an mpsc::channel to combine stdin events with app events
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();

        thread::spawn(move || input_thread(event_tx));
        thread::spawn(background_task);

        self.run(terminal, rx)
    }

    /// Main application loop
    fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        rx: mpsc::Receiver<Event>,
    ) -> anyhow::Result<()> {
        for event in rx {
            if self.handle_ui_event(event) {
                break;
            }
            self.draw(terminal)?;
        }
        Ok(())
    }

    fn handle_ui_event(&mut self, event: Event) -> bool {
        debug!(target: "App", "Handling UI event: {:?}",event);
        if let Event::Key(key) = event {
            match key.code {
                Key::Char('q') => return true,
                Key::Char(' ') => self.state.transition(TuiWidgetEvent::SpaceKey),
                Key::Esc => self.state.transition(TuiWidgetEvent::EscapeKey),
                Key::PageUp => self.state.transition(TuiWidgetEvent::PrevPageKey),
                Key::PageDown => self.state.transition(TuiWidgetEvent::NextPageKey),
                Key::Up => self.state.transition(TuiWidgetEvent::UpKey),
                Key::Down => self.state.transition(TuiWidgetEvent::DownKey),
                Key::Left => self.state.transition(TuiWidgetEvent::LeftKey),
                Key::Right => self.state.transition(TuiWidgetEvent::RightKey),
                Key::Char('+') => self.state.transition(TuiWidgetEvent::PlusKey),
                Key::Char('-') => self.state.transition(TuiWidgetEvent::MinusKey),
                Key::Char('h') => self.state.transition(TuiWidgetEvent::HideKey),
                Key::Char('f') => self.state.transition(TuiWidgetEvent::FocusKey),
                _ => (),
            }
        }
        false
    }

    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        terminal.draw(|frame| {
            frame.render_widget(self, frame.area());
        })?;
        Ok(())
    }
}

/// A background task that logs a log entry for each log level every second.
fn background_task() {
    loop {
        error!(target:"background-task", "an error");
        warn!(target:"background-task", "a warning");
        info!(target:"background-task", "a two line info\nsecond line");
        debug!(target:"background-task", "a debug");
        trace!(target:"background-task", "a trace");
        thread::sleep(time::Duration::from_millis(1000));
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [smart_area, help_area] =
            Layout::vertical([Constraint::Fill(50), Constraint::Length(3)]).areas(area);

        TuiLoggerSmartWidget::default()
            .style_error(Style::default().fg(Color::Red))
            .style_debug(Style::default().fg(Color::Green))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_trace(Style::default().fg(Color::Magenta))
            .style_info(Style::default().fg(Color::Cyan))
            .output_separator(':')
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(true)
            .output_line(true)
            .state(&self.state)
            .render(smart_area, buf);

        if area.width > 40 {
            Text::from(vec![
                "Q: Quit | Tab: Switch state | ↑/↓: Select target | f: Focus target".into(),
                "←/→: Display level | +/-: Filter level | Space: Toggle hidden targets".into(),
                "h: Hide target selector | PageUp/Down: Scroll | Esc: Cancel scroll".into(),
            ])
            .style(Color::Gray)
            .centered()
            .render(help_area, buf);
        }
    }
}

/// A module for crossterm specific code
#[cfg(feature = "crossterm")]
mod crossterm_backend {
    use super::*;

    pub use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode as Key},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };

    pub fn init_terminal() -> io::Result<Terminal<impl Backend>> {
        trace!(target:"crossterm", "Initializing terminal");
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(io::stdout());
        Terminal::new(backend)
    }

    pub fn restore_terminal() -> io::Result<()> {
        trace!(target:"crossterm", "Restoring terminal");
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
    }

    pub fn input_thread(tx_event: mpsc::Sender<Event>) -> anyhow::Result<()> {
        trace!(target:"crossterm", "Starting input thread");
        while let Ok(event) = event::read() {
            trace!(target:"crossterm", "Stdin event received {:?}", event);
            tx_event.send(event)?;
        }
        Ok(())
    }
}
