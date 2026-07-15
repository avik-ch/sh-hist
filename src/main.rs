mod history;
mod render;
mod search;

use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use history::{HistoryEntry, load_zsh_history};
use ratatui::{DefaultTerminal, TerminalOptions, Viewport};
use search::search_history;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

// CONFIG VARIABLES
const MAX_RESULTS: usize = 10;
const SHOW_DUPLICATE_COMMANDS: bool = false;
const SEARCH_HISTORY_METADATA: bool = false;

// shell widget flags
const EXECUTE_SELECTED_EXIT_STATUS: i32 = 10;
const EDIT_SELECTED_EXIT_STATUS: i32 = 11;

fn main() -> Result<()> {
    let result_file = parse_result_file()?;
    color_eyre::install()?;
    let app = App::new()?;

    // Initialize terminal input before Ratatui's inline viewport queries the cursor position.
    event::poll(Duration::ZERO)?;
    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline((MAX_RESULTS + 1) as u16),
    });

    let app_exit = app.run(&mut terminal)?;
    ratatui::restore();

    if let (Some(path), AppExit::Selected { action, command }) = (result_file, app_exit) {
        std::fs::write(path, command)?;
        std::process::exit(action.exit_status());
    }

    Ok(())
}

fn parse_result_file() -> Result<Option<PathBuf>> {
    let mut arguments = env::args_os().skip(1);
    let Some(flag) = arguments.next() else {
        return Ok(None);
    };

    if flag == "--help" || flag == "-h" {
        println!("Usage: sh-hist [--result-file <path>]");
        std::process::exit(0);
    }

    if flag != "--result-file" {
        color_eyre::eyre::bail!("unknown argument: {}", flag.to_string_lossy());
    }

    let Some(path) = arguments.next() else {
        color_eyre::eyre::bail!("--result-file requires a path");
    };

    if arguments.next().is_some() {
        color_eyre::eyre::bail!("only --result-file <path> is supported");
    }

    Ok(Some(path.into()))
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

#[derive(Debug, PartialEq, Eq)]
enum AppExit {
    Cancelled,
    Selected {
        action: SelectionAction,
        command: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum SelectionAction {
    Execute,
    Edit,
}

impl SelectionAction {
    fn exit_status(&self) -> i32 {
        match self {
            Self::Execute => EXECUTE_SELECTED_EXIT_STATUS,
            Self::Edit => EDIT_SELECTED_EXIT_STATUS,
        }
    }
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

    fn selected_command(&self) -> Option<&str> {
        let history_index = *self.matches.get(self.match_window_start)?;
        Some(&self.history.get(history_index)?.command)
    }

    fn select(&self, action: SelectionAction) -> Option<AppExit> {
        Some(AppExit::Selected {
            action,
            command: self.selected_command()?.to_string(),
        })
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<AppExit> {
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
                        KeyCode::Char('e') | KeyCode::Char('i') | KeyCode::Char('a') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(AppExit::Cancelled);
                        }
                        KeyCode::Char('j') | KeyCode::Down => self.move_selection_down(),
                        KeyCode::Char('k') | KeyCode::Up => self.move_selection_up(),
                        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.move_selection_down()
                        }
                        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.move_selection_up()
                        }
                        KeyCode::Char(digit @ '0'..='9') => {
                            self.jump_to_result(digit.to_digit(10).unwrap() as usize);
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => {
                            if let Some(app_exit) = self.select(SelectionAction::Execute) {
                                return Ok(app_exit);
                            }
                        }
                        KeyCode::Tab => {
                            if let Some(app_exit) = self.select(SelectionAction::Edit) {
                                return Ok(app_exit);
                            }
                        }
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        KeyCode::Down => self.move_selection_down(),
                        KeyCode::Up => self.move_selection_up(),
                        KeyCode::Char('j') | KeyCode::Char('n')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            self.move_selection_down();
                        }
                        KeyCode::Char('k') | KeyCode::Char('p')
                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
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
    fn selected_command_uses_the_highlighted_match() {
        let app = App {
            input: Input::default(),
            input_mode: InputMode::default(),
            history: vec![
                HistoryEntry::new("newest".to_string(), String::new(), false),
                HistoryEntry::new("selected\ncommand".to_string(), String::new(), false),
            ],
            matches: vec![1, 0],
            match_window_start: 0,
        };

        assert_eq!(app.selected_command(), Some("selected\ncommand"));
        assert_eq!(
            app.select(SelectionAction::Edit),
            Some(AppExit::Selected {
                action: SelectionAction::Edit,
                command: "selected\ncommand".to_string(),
            })
        );
    }

    #[test]
    fn selection_is_unavailable_without_matches() {
        let app = App {
            input: Input::default(),
            input_mode: InputMode::default(),
            history: Vec::new(),
            matches: Vec::new(),
            match_window_start: 0,
        };

        assert_eq!(app.select(SelectionAction::Execute), None);
    }

    #[test]
    fn selection_actions_use_distinct_exit_statuses() {
        assert_eq!(
            SelectionAction::Execute.exit_status(),
            EXECUTE_SELECTED_EXIT_STATUS
        );
        assert_eq!(
            SelectionAction::Edit.exit_status(),
            EDIT_SELECTED_EXIT_STATUS
        );
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
