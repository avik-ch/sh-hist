use color_eyre::Result;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub command: String,
    pub metadata: String,
    pub searchable_text: String,
}

impl HistoryEntry {
    pub fn new(command: String, metadata: String, search_history_metadata: bool) -> Self {
        let searchable_text = if search_history_metadata && !metadata.is_empty() {
            format!("{metadata} {command}")
        } else {
            command.clone()
        };

        Self {
            command,
            metadata,
            searchable_text,
        }
    }
}

pub fn load_zsh_history(
    show_duplicate_commands: bool,
    search_history_metadata: bool,
) -> Result<Vec<HistoryEntry>> {
    let zsh_history_path = env::var_os("HISTFILE")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".zsh_history")));

    let Some(path) = zsh_history_path else {
        return Ok(Vec::new());
    };
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    Ok(filter_duplicate_commands(
        parse_zsh_history(&contents, search_history_metadata),
        show_duplicate_commands,
    ))
}

pub fn parse_zsh_history(contents: &str, search_history_metadata: bool) -> Vec<HistoryEntry> {
    let mut entries = Vec::new();
    let mut current_command = None;
    let mut current_metadata = String::new();

    for line in contents.lines() {
        if let Some((metadata, command)) = parse_extended_history_line(line) {
            push_history_entry(
                &mut entries,
                current_command.take(),
                &current_metadata,
                search_history_metadata,
            );
            current_command = Some(command.to_string());
            current_metadata = metadata.to_string();
        } else if let Some(command) = current_command.as_mut() {
            command.push('\n');
            command.push_str(line);
        } else if !line.is_empty() {
            entries.push(HistoryEntry::new(
                line.to_string(),
                String::new(),
                search_history_metadata,
            ));
        }
    }

    push_history_entry(
        &mut entries,
        current_command.take(),
        &current_metadata,
        search_history_metadata,
    );
    entries.reverse();
    entries
}

fn parse_extended_history_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix(": ")?;
    let (timestamp, rest) = rest.split_once(':')?;
    if timestamp.is_empty()
        || !timestamp
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return None;
    }

    let (duration, command) = rest.split_once(';')?;
    if duration.is_empty() || !duration.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }

    Some((line.split_once(';')?.0, command))
}

fn push_history_entry(
    entries: &mut Vec<HistoryEntry>,
    command: Option<String>,
    metadata: &str,
    search_history_metadata: bool,
) {
    if let Some(command) = command.filter(|command| !command.is_empty()) {
        entries.push(HistoryEntry::new(
            command,
            metadata.to_string(),
            search_history_metadata,
        ));
    }
}

pub fn filter_duplicate_commands(
    entries: Vec<HistoryEntry>,
    show_duplicates: bool,
) -> Vec<HistoryEntry> {
    if show_duplicates {
        return entries;
    }

    let mut seen = HashSet::new();
    entries
        .into_iter()
        .filter(|entry| seen.insert(entry.command.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extended_zsh_history_newest_first() {
        let entries = parse_zsh_history(": 100:0;git status\n: 101:2;cargo test\n", false);

        assert_eq!(entries[0].command, "cargo test");
        assert_eq!(entries[0].metadata, ": 101:2");
        assert_eq!(entries[1].command, "git status");
    }

    #[test]
    fn parses_plain_history_newest_first() {
        let entries = parse_zsh_history("git status\ncargo check\n", false);

        assert_eq!(entries[0].command, "cargo check");
        assert_eq!(entries[1].command, "git status");
    }

    #[test]
    fn preserves_multiline_extended_commands() {
        let entries = parse_zsh_history(
            ": 100:0;for file in *; do\n  echo $file\ndone\n: 101:0;git log\n",
            false,
        );

        assert_eq!(entries[1].command, "for file in *; do\n  echo $file\ndone");
    }

    #[test]
    fn duplicate_filter_keeps_newest_entry() {
        let entries = vec![
            HistoryEntry::new("git status".to_string(), ": 101:0".to_string(), false),
            HistoryEntry::new("cargo check".to_string(), ": 100:0".to_string(), false),
            HistoryEntry::new("git status".to_string(), ": 99:0".to_string(), false),
        ];

        let filtered = filter_duplicate_commands(entries, false);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].metadata, ": 101:0");
    }

    #[test]
    fn searchable_text_can_include_metadata() {
        let entry = HistoryEntry::new("git status".to_string(), ": 101:2".to_string(), true);

        assert_eq!(entry.searchable_text, ": 101:2 git status");
    }
}
