//! Markdown link destinations outside matching fenced code blocks.

use regex::Regex;

/// Find inline, image and reference-definition destinations for structural checks.
pub fn links(content: &str) -> Vec<String> {
    let mut fence: Option<(char, usize)> = None;
    let mut text = String::new();
    for line in content.lines() {
        let trimmed = line.trim_start_matches(' ');
        let indentation = line.len() - trimmed.len();
        if indentation <= 3 {
            if let Some(delimiter @ ('`' | '~')) = trimmed.chars().next() {
                let count = trimmed
                    .chars()
                    .take_while(|character| *character == delimiter)
                    .count();
                if count >= 3 {
                    match fence {
                        None => fence = Some((delimiter, count)),
                        Some((opening, length))
                            if opening == delimiter
                                && count >= length
                                && trimmed.chars().skip(count).all(char::is_whitespace) =>
                        {
                            fence = None
                        }
                        _ => {}
                    }
                    continue;
                }
            }
        }
        if fence.is_none() {
            text.push_str(line);
            text.push('\n');
        }
    }
    let patterns = [
        r#"\]\(\s*(<[^>\n]+>|[^\s)]+)(?:\s+['\"][^\n]*?['\"])?\s*\)"#,
        r"(?m)^\s{0,3}\[[^\]\n]+\]:\s*(<[^>\n]+>|\S+)",
    ];
    let mut targets = Vec::new();
    for pattern in patterns {
        if let Ok(regex) = Regex::new(pattern) {
            targets.extend(
                regex
                    .captures_iter(&text)
                    .filter_map(|capture| capture.get(1))
                    .map(|value| value.as_str().trim_matches(['<', '>']).to_owned()),
            );
        }
    }
    targets
}
