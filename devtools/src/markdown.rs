//! Markdown link destinations outside matching fenced code blocks.

use regex::Regex;
use std::sync::OnceLock;

/// Find inline, image and reference-definition destinations for structural checks.
fn prose(content: &str) -> String {
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
    text
}

/// Find inline, image and reference-definition destinations outside fences.
pub fn links(content: &str) -> Vec<String> {
    let text = prose(content);
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

/// Anchors for the documented local Markdown subset (ATX/setext and explicit HTML).
pub fn anchors(content: &str) -> std::collections::BTreeSet<String> {
    use std::collections::BTreeSet;
    let text = prose(content);
    let mut anchors = BTreeSet::new();
    let html =
        Regex::new(r#"(?i)<(?:a|[a-z][a-z0-9]*)\b[^>]*\b(?:id|name)\s*=\s*["']([^"']+)["'][^>]*>"#)
            .expect("static Markdown expression");
    for c in html.captures_iter(&text) {
        anchors.insert(c[1].into());
    }
    let inline_link =
        Regex::new(r"!?\[([^\]]+)\](?:\([^)]*\)|\[[^\]]*\])").expect("static Markdown expression");
    let tag = Regex::new(r"<[^>]*>").expect("static Markdown expression");
    let emphasis = Regex::new(r"(^|\s)_([^_]+)_(\s|$)").expect("static Markdown expression");
    let mut previous = "";
    for line in text.lines() {
        let trimmed = line.trim();
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        let heading =
            if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(char::is_whitespace) {
                Some(trimmed[hashes..].trim().trim_end_matches('#').trim_end())
            } else if !previous.trim().is_empty()
                && trimmed.len() >= 2
                && (trimmed.chars().all(|c| c == '=') || trimmed.chars().all(|c| c == '-'))
            {
                Some(previous.trim())
            } else {
                None
            };
        if let Some(heading) = heading {
            let heading = inline_link.replace_all(heading, "$1");
            let heading = tag.replace_all(&heading, "");
            let heading = emphasis.replace_all(&heading, "$1$2$3");
            let heading = heading
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'");
            let slug: String = heading
                .to_lowercase()
                .chars()
                .filter_map(|c| {
                    if c.is_whitespace() {
                        Some('-')
                    } else if c.is_alphanumeric() || c == '-' || c == '_' {
                        Some(c)
                    } else {
                        None
                    }
                })
                .collect();
            let mut unique = slug.clone();
            let mut suffix = 0;
            while anchors.contains(&unique) {
                suffix += 1;
                unique = format!("{slug}-{suffix}");
            }
            anchors.insert(unique);
        }
        previous = line;
    }
    anchors
}

/// Validate a local Markdown fragment after the existing boundary/path check.
pub fn check_anchor(
    source: &std::path::Path,
    target: Option<&std::path::Path>,
    raw: &str,
) -> Result<(), String> {
    if raw.split_once(':').is_some_and(|(s, _)| {
        ["http", "https", "mailto"]
            .iter()
            .any(|x| s.eq_ignore_ascii_case(x))
    }) {
        return Ok(());
    }
    let Some((_, fragment)) = raw.split_once('#') else {
        return Ok(());
    };
    if fragment.is_empty() {
        return Ok(());
    }
    let target = target.unwrap_or(source);
    if target.extension().is_none_or(|e| e != "md") {
        return Ok(());
    }
    let mut decoded = Vec::new();
    let mut bytes = fragment.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let a = bytes.next().and_then(|b| (b as char).to_digit(16));
            let b = bytes.next().and_then(|b| (b as char).to_digit(16));
            match (a, b) {
                (Some(a), Some(b)) => decoded.push((a * 16 + b) as u8),
                _ => return Err(format!("invalid anchor encoding: {raw}")),
            }
        } else {
            decoded.push(b);
        }
    }
    let fragment =
        String::from_utf8(decoded).map_err(|_| format!("invalid anchor encoding: {raw}"))?;
    let content = std::fs::read_to_string(target).map_err(|e| e.to_string())?;
    if !anchors(&content).contains(&fragment) {
        return Err(format!("missing local anchor: {raw}"));
    }
    Ok(())
}
