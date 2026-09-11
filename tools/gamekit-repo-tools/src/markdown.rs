//! Markdown link destinations outside matching fenced code blocks.

use regex::Regex;
use std::sync::OnceLock;

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
    static PATTERNS: OnceLock<[Regex; 2]> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        [
            r#"\]\(\s*(<[^>\n]+>|[^\s)]+)(?:\s+['\"][^\n]*?['\"])?\s*\)"#,
            r"(?m)^\s{0,3}\[[^\]\n]+\]:\s*(<[^>\n]+>|\S+)",
        ]
        .map(|pattern| {
            Regex::new(pattern).expect("static Markdown destination expression is valid")
        })
    });
    let mut targets = Vec::new();
    for regex in patterns {
        targets.extend(
            regex
                .captures_iter(&text)
                .filter_map(|capture| capture.get(1))
                .map(|value| value.as_str().trim_matches(['<', '>']).to_owned()),
        );
    }
    targets
}
