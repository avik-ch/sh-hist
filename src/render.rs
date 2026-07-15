use crate::InputMode;
use crate::history::HistoryEntry;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem, Paragraph};
use tui_input::Input;

const INPUT_PREFIX: &str = "> ";
const INPUT_COLOR: Color = Color::Blue;
const MULTILINE_PREVIEW_MAX_CHARS: usize = 120;

pub fn render(
    frame: &mut Frame,
    input: &Input,
    input_mode: &InputMode,
    history: &[HistoryEntry],
    matches: &[usize],
    match_window_start: usize,
    max_results: usize,
) {
    let area = frame.area();
    let input_line = Rect { height: 1, ..area };
    let prefix_style = match input_mode {
        InputMode::Normal => Style::default(),
        InputMode::Editing => Style::default().fg(INPUT_COLOR),
    };
    frame.render_widget(Line::styled(INPUT_PREFIX, prefix_style), input_line);

    let prefix_width = INPUT_PREFIX.len() as u16;
    let input_area = Rect {
        x: input_line.x + prefix_width,
        width: input_line.width.saturating_sub(prefix_width),
        ..input_line
    };
    let width = input_area.width as usize;
    let scroll = input.visual_scroll(width);
    let input_widget = Paragraph::new(input.value())
        .style(match input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(INPUT_COLOR),
        })
        .scroll((0, scroll as u16));
    frame.render_widget(input_widget, input_area);

    let results_area = Rect {
        y: area.y.saturating_add(1),
        height: area.height.saturating_sub(1),
        ..area
    };
    render_results(
        results_area,
        frame,
        history,
        matches,
        match_window_start,
        max_results,
    );

    if *input_mode == InputMode::Editing {
        let x = input.visual_cursor().max(scroll) - scroll;
        frame.set_cursor_position((input_area.x + x as u16, input_area.y));
    }
}

fn render_results(
    area: Rect,
    frame: &mut Frame,
    history: &[HistoryEntry],
    matches: &[usize],
    match_window_start: usize,
    max_results: usize,
) {
    let Some(visible_matches) = matches.get(match_window_start..) else {
        frame.render_widget(
            Paragraph::new("No matches").style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    };

    if visible_matches.is_empty() {
        frame.render_widget(
            Paragraph::new("No matches").style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }

    let items = visible_matches
        .iter()
        .take(max_results)
        .enumerate()
        .map(|(result_index, history_index)| {
            let preview = command_preview(&history[*history_index].command);
            let style = if result_index == 0 {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(format!("{result_index} {preview}")).style(style)
        })
        .collect::<Vec<_>>();

    frame.render_widget(List::new(items), area);
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
