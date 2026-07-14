use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame, TerminalOptions};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: ratatui::Viewport::Inline(10),
    });

    ratatui::run(|terminal| App::default().run(terminal))
}

/// App holds the state of the application
#[derive(Debug, Default)]
struct App {
    /// Current value of the input box
    input: Input,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
}

#[derive(Debug, Default, PartialEq)]
enum InputMode {
    #[default]
    Editing,
    Normal,
}

impl App {
    fn submit_message(&mut self) {
        self.messages.push(self.input.value_and_reset());
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
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => self.submit_message(),
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {
                            self.input.handle_event(&event);
                        }
                    },
                    InputMode::Editing => {}
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Min(1),    // messages
            Constraint::Length(3), // input area
        ]);
        let [messages_area, input_area] = frame.area().layout(&layout);

        // HISTORY
        let history: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = Line::from(Span::raw(format!("{i}: {m}")));
                ListItem::new(content)
            })
            .collect();
        let messages = List::new(history).block(Block::bordered().title("Messages"));
        frame.render_widget(messages, messages_area);

        // INPUT AREA
        let width = input_area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let input = Paragraph::new(self.input.value())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);

        if self.input_mode == InputMode::Editing {
            // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
            // end of the input text and one line down from the border to the input line
            let x = self.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((input_area.x + x as u16, input_area.y + 1))
        }
    }
}
