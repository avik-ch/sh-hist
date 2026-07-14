use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame, TerminalOptions, Viewport};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

const INPUT_PREFIX: &str = "> ";

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(1),
    });

    let result = App::default().run(&mut terminal);
    ratatui::restore();
    result
}

/// App holds the state of the application
#[derive(Debug, Default)]
struct App {
    /// Current value of the input box
    input: Input,
    /// Current input mode
    input_mode: InputMode,
}

#[derive(Debug, Default, PartialEq)]
enum InputMode {
    #[default]
    Editing,
    Normal,
}

impl App {
    fn submit_message(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let message = self.input.value_and_reset();
        terminal.insert_before(1, |buf| {
            Paragraph::new(message).render(buf.area, buf);
        })?;
        Ok(())
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
                        KeyCode::Enter => self.submit_message(terminal)?,
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
        let area = frame.area();
        frame.render_widget(INPUT_PREFIX, area);

        let prefix_width = INPUT_PREFIX.len() as u16;
        let input_area = Rect {
            x: area.x.saturating_add(prefix_width),
            width: area.width.saturating_sub(prefix_width),
            ..area
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

        if self.input_mode == InputMode::Editing {
            let x = self.input.visual_cursor().max(scroll) - scroll;
            frame.set_cursor_position((input_area.x + x as u16, input_area.y))
        }
    }
}
