use crate::history::HistoryEntry;
use std::collections::HashSet;

pub fn search_history(history: &[HistoryEntry], query: &str, max_results: usize) -> Vec<usize> {
    if query.is_empty() {
        return (0..history.len().min(max_results)).collect();
    }

    let mut matches = Vec::new();
    let mut seen = HashSet::new();

    for (index, entry) in history.iter().enumerate() {
        if normal_matches(query, &entry.searchable_text) {
            matches.push(index);
            seen.insert(index);

            if matches.len() >= max_results {
                return matches;
            }
        }
    }

    for (index, entry) in history.iter().enumerate() {
        if !seen.contains(&index) && fuzzy_matches(query, &entry.searchable_text) {
            matches.push(index);

            if matches.len() >= max_results {
                break;
            }
        }
    }

    matches
}

fn normal_matches(query: &str, candidate: &str) -> bool {
    candidate.to_lowercase().contains(&query.to_lowercase())
}

fn fuzzy_matches(query: &str, candidate: &str) -> bool {
    let mut query_chars = query.to_lowercase().chars().collect::<Vec<_>>().into_iter();
    let Some(mut query_char) = query_chars.next() else {
        return true;
    };

    for candidate_char in candidate.to_lowercase().chars() {
        if candidate_char == query_char {
            let Some(next_query_char) = query_chars.next() else {
                return true;
            };
            query_char = next_query_char;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(command: &str) -> HistoryEntry {
        HistoryEntry::new(command.to_string(), String::new(), false)
    }

    #[test]
    fn empty_query_returns_newest_commands() {
        let history = (0..12)
            .map(|index| entry(&format!("command {index}")))
            .collect::<Vec<_>>();

        assert_eq!(
            search_history(&history, "", 10),
            (0..10).collect::<Vec<_>>()
        );
    }

    #[test]
    fn normal_matches_come_before_fuzzy_matches() {
        let history = vec![entry("go sleep"), entry("git stash"), entry("gs")];

        assert_eq!(search_history(&history, "gs", 10), vec![2, 0, 1]);
    }

    #[test]
    fn fuzzy_results_exclude_normal_matches() {
        let history = vec![entry("git status"), entry("gs"), entry("go sleep")];

        assert_eq!(search_history(&history, "gs", 10), vec![1, 0, 2]);
    }

    #[test]
    fn search_is_case_insensitive() {
        let history = vec![entry("Cargo Check")];

        assert_eq!(search_history(&history, "cargo", 10), vec![0]);
    }

    #[test]
    fn result_count_is_capped() {
        let history = (0..12)
            .map(|index| entry(&format!("cargo command {index}")))
            .collect::<Vec<_>>();

        assert_eq!(search_history(&history, "cargo", 3), vec![0, 1, 2]);
    }
}
