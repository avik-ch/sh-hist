mod history;
mod render;
mod search;

use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use history::{HistoryEntry, load_zsh_history};
use ratatui::{DefaultTerminal, TerminalOptions, Viewport};
use search::search_history;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

// CONFIG VARIABLES
const MAX_RESULTS: usize = 10;
const SHOW_DUPLICATE_COMMANDS: bool = false;
const SEARCH_HISTORY_METADATA: bool = false;

fn main() -> Result<()> {
    color_eyre::install()?;
    let app = App::new()?;

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline((MAX_RESULTS + 1) as u16),
    });

    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}

/// App holds the state of the application
#[derive(Debug)]
struct App {
    /// Current value of the input box
    input: Input,
    /// Current input mode
    input_mode: InputMode,
    /// Parsed shell history, newest command first.
    history: Vec<HistoryEntry>,
    /// All matching history indexes in search-rank order.
    matches: Vec<usize>,
    /// Offset of the highlighted command within matches.
    match_window_start: usize,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) enum InputMode {
    #[default]
    Editing,
    Normal,
}

impl App {
    fn new() -> Result<Self> {
        let history = load_zsh_history(SHOW_DUPLICATE_COMMANDS, SEARCH_HISTORY_METADATA)?;
        let mut app = Self {
            input: Input::default(),
            input_mode: InputMode::default(),
            history,
            matches: Vec::new(),
            match_window_start: 0,
        };
        app.update_matches();
        Ok(app)
    }

    fn update_matches(&mut self) {
        self.matches = search_history(&self.history, self.input.value(), self.history.len());
        self.match_window_start = 0;
    }

    fn move_selection_up(&mut self) {
        self.match_window_start = self.match_window_start.saturating_sub(1);
    }

    fn move_selection_down(&mut self) {
        if self.match_window_start < self.matches.len().saturating_sub(1) {
            self.match_window_start += 1;
        }
    }

    fn jump_to_result(&mut self, distance: usize) {
        let Some(window_start) = self.match_window_start.checked_add(distance) else {
            return;
        };

        if window_start < self.matches.len() {
            self.match_window_start = window_start;
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| {
                render::render(
                    frame,
                    &self.input,
                    &self.input_mode,
                    &self.history,
                    &self.matches,
                    self.match_window_start,
                    MAX_RESULTS,
                )
            })?;

            let event = event::read()?;
            if let Some(key) = event.as_key_press_event() {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(());
                        }
                        KeyCode::Char('j') | KeyCode::Down => self.move_selection_down(),
                        KeyCode::Char('k') | KeyCode::Up => self.move_selection_up(),
                        KeyCode::Char(digit @ '0'..='9') => {
                            self.jump_to_result(digit.to_digit(10).unwrap() as usize);
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter | KeyCode::Tab => {
                            // Will handle later
                        }
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        KeyCode::Down => self.move_selection_down(),
                        KeyCode::Up => self.move_selection_up(),
                        KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.move_selection_down();
                        }
                        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.move_selection_up();
                        }
                        _ => {
                            self.input.handle_event(&event);
                            self.update_matches();
                        }
                    },
                    InputMode::Editing => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_returns_newest_commands() {
        let mut app = App {
            input: Input::default(),
            input_mode: InputMode::default(),
            history: (0..12)
                .map(|index| HistoryEntry::new(format!("command {index}"), String::new(), false))
                .collect(),
            matches: Vec::new(),
            match_window_start: 0,
        };

        app.update_matches();

        assert_eq!(app.matches, (0..12).collect::<Vec<_>>());
        assert_eq!(app.match_window_start, 0);
    }

    #[test]
    fn result_window_stays_within_match_boundaries() {
        let mut app = App {
            input: Input::default(),
            input_mode: InputMode::default(),
            history: Vec::new(),
            matches: vec![0, 1],
            match_window_start: 0,
        };

        app.move_selection_up();
        assert_eq!(app.match_window_start, 0);

        app.move_selection_down();
        app.move_selection_down();
        assert_eq!(app.match_window_start, 1);
    }

    #[test]
    fn numeric_selection_moves_the_result_window() {
        let mut app = App {
            input: Input::default(),
            input_mode: InputMode::default(),
            history: Vec::new(),
            matches: (0..12).collect(),
            match_window_start: 2,
        };

        app.jump_to_result(4);
        assert_eq!(app.match_window_start, 6);

        app.jump_to_result(9);
        assert_eq!(app.match_window_start, 6);
    }

    #[test]
    fn updating_matches_resets_result_window_to_first_match() {
        let mut app = App {
            input: Input::default(),
            input_mode: InputMode::default(),
            history: (0..3)
                .map(|index| HistoryEntry::new(format!("command {index}"), String::new(), false))
                .collect(),
            matches: vec![1, 2],
            match_window_start: 1,
        };

        app.update_matches();

        assert_eq!(app.match_window_start, 0);
    }
}
