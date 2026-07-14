mod history;
mod search;

use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};
use history::{HistoryEntry, load_zsh_history};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame, TerminalOptions, Viewport};
use search::search_history;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

// CONFIG VARIABLES
const INPUT_PREFIX: &str = "> ";
const MAX_RESULTS: usize = 10;
const SHOW_DUPLICATE_COMMANDS: bool = false;
const SEARCH_HISTORY_METADATA: bool = false;
const MULTILINE_PREVIEW_MAX_CHARS: usize = 120;

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
    /// Matching history indexes, newest first and capped to MAX_RESULTS.
    matches: Vec<usize>,
}

#[derive(Debug, Default, PartialEq)]
enum InputMode {
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
        };
        app.update_matches();
        Ok(app)
    }

    fn update_matches(&mut self) {
        self.matches = search_history(&self.history, self.input.value(), MAX_RESULTS);
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

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
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter | KeyCode::Tab => {
                            // Will handle later
                        }
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
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

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let input_line = Rect { height: 1, ..area };
        frame.render_widget(INPUT_PREFIX, input_line);

        let prefix_width = INPUT_PREFIX.len() as u16;
        let input_area = Rect {
            x: input_line.x.saturating_add(prefix_width),
            width: input_line.width.saturating_sub(prefix_width),
            ..input_line
        };
        let width = input_area.width as usize;
        let scroll = self.input.visual_scroll(width);
        let input = Paragraph::new(self.input.value())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .scroll((0, scroll as u16));
        frame.render_widget(input, input_area);

        let results_area = Rect {
            y: area.y.saturating_add(1),
            height: area.height.saturating_sub(1),
            ..area
        };
        self.render_results(results_area, frame);

        if self.input_mode == InputMode::Editing {
            let x = self.input.visual_cursor().max(scroll) - scroll;
            frame.set_cursor_position((input_area.x + x as u16, input_area.y))
        }
    }

    fn render_results(&self, area: Rect, frame: &mut Frame) {
        if self.matches.is_empty() {
            frame.render_widget(
                Paragraph::new("No matches").style(Style::default().fg(Color::DarkGray)),
                area,
            );
            return;
        }

        let items = self
            .matches
            .iter()
            .enumerate()
            .map(|(result_index, history_index)| {
                let preview = command_preview(&self.history[*history_index].command);
                let style = if result_index == 0 {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                ListItem::new(preview).style(style)
            })
            .collect::<Vec<_>>();

        frame.render_widget(List::new(items), area);
    }
}

fn command_preview(command: &str) -> String {
    let is_multiline = command.contains(['\n', '\r']);
    let compact = command.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = truncate_with_ellipsis(&compact, MULTILINE_PREVIEW_MAX_CHARS);

    if is_multiline && !preview.ends_with("...") {
        preview = truncate_with_ellipsis(&format!("{preview}..."), MULTILINE_PREVIEW_MAX_CHARS);
    }

    preview
}

fn truncate_with_ellipsis(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let mut truncated = value
        .chars()
        .take(max_chars - 3)
        .collect::<String>()
        .trim_end()
        .to_string();
    truncated.push_str("...");
    truncated
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
        };

        app.update_matches();

        assert_eq!(app.matches, (0..MAX_RESULTS).collect::<Vec<_>>());
    }

    #[test]
    fn multiline_preview_compacts_and_marks_omission() {
        let preview = command_preview("for file in *; do\n  echo $file\ndone");

        assert_eq!(preview, "for file in *; do echo $file done...");
    }

    #[test]
    fn long_preview_is_truncated_with_ellipsis() {
        let preview = command_preview(&"x".repeat(MULTILINE_PREVIEW_MAX_CHARS + 1));

        assert_eq!(preview.chars().count(), MULTILINE_PREVIEW_MAX_CHARS);
        assert!(preview.ends_with("..."));
    }
}
