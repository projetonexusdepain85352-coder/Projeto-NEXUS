use std::collections::HashSet;

fn normalize_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn looks_like_code_line(line: &str) -> bool {
    let markers = [
        "::", "->", "=>", "{", "}", "(", ")", "[", "]", "=", "fn ", "let ", "pub ", "impl ",
        "trait ", "struct ", "enum ", "#include", "return ",
    ];
    markers.iter().any(|m| line.contains(m))
}

fn looks_like_toc_line(lower: &str) -> bool {
    if lower.len() > 140 {
        return false;
    }

    let bytes = lower.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() && bytes[i] == b'.' {
        let mut groups = 1usize;
        let mut j = i + 1;
        while j < bytes.len() {
            let start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j == start {
                break;
            }
            if j < bytes.len() && bytes[j] == b'.' {
                groups += 1;
                j += 1;
                continue;
            }
            break;
        }
        if groups >= 2 {
            return true;
        }
    }

    if (lower.starts_with("- ") || lower.starts_with("* ")) && lower.split_whitespace().count() <= 6
    {
        return true;
    }

    false
}

fn drop_by_signature(lower: &str) -> bool {
    const EXACT: &[&str] = &[
        "table of contents",
        "on this page",
        "skip to main content",
        "next page",
        "previous page",
        "search",
        "search docs",
        "edit this page",
        "back to top",
        "copyright",
        "all rights reserved",
        "privacy policy",
        "terms of service",
        "navigation",
        "menu",
    ];

    if EXACT.contains(&lower) {
        return true;
    }

    let starts = [
        "was this page helpful",
        "last updated",
        "report an issue",
        "open an issue",
        "view source",
        "download pdf",
    ];
    if starts.iter().any(|x| lower.starts_with(x)) {
        return true;
    }

    if lower.contains("cookie") && lower.contains("policy") {
        return true;
    }

    false
}

fn push_line(lines: &mut Vec<String>, line: &str, last_blank: &mut bool) {
    if line.trim().is_empty() {
        if !*last_blank {
            lines.push(String::new());
            *last_blank = true;
        }
    } else {
        lines.push(line.to_string());
        *last_blank = false;
    }
}

fn clean_pass(raw: &str, min_line_len: usize) -> String {
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut last_blank = false;

    for raw_line in normalized.lines() {
        let trimmed = raw_line.trim();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            push_line(&mut out, trimmed, &mut last_blank);
            continue;
        }

        if trimmed.is_empty() {
            push_line(&mut out, "", &mut last_blank);
            continue;
        }

        if in_code_block {
            push_line(&mut out, raw_line, &mut last_blank);
            continue;
        }

        let lowered = normalize_spaces(trimmed).to_lowercase();

        if drop_by_signature(&lowered) || looks_like_toc_line(&lowered) {
            continue;
        }

        if trimmed.chars().count() < min_line_len && !looks_like_code_line(trimmed) {
            continue;
        }

        let dedupe_key = lowered.clone();
        if !seen.insert(dedupe_key) {
            continue;
        }

        push_line(&mut out, trimmed, &mut last_blank);
    }

    let mut result = out.join("\n");
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    result.trim().to_string()
}

pub fn clean_document_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let strict = clean_pass(trimmed, 20);
    let raw_len = trimmed.chars().count();
    let strict_len = strict.chars().count();

    if strict_len >= 400 && strict_len * 8 >= raw_len {
        return strict;
    }

    let soft = clean_pass(trimmed, 10);
    let soft_len = soft.chars().count();
    if soft_len >= 200 && soft_len * 10 >= raw_len {
        return soft;
    }

    soft
}

#[cfg(test)]
mod tests {
    use super::clean_document_text;

    #[test]
    fn removes_navigation_signatures() {
        let raw = "Table of contents\nSkip to main content\nRust ownership rules are enforced by the borrow checker.";
        let cleaned = clean_document_text(raw);
        assert!(!cleaned.to_lowercase().contains("table of contents"));
        assert!(!cleaned.to_lowercase().contains("skip to main content"));
        assert!(cleaned.contains("borrow checker"));
    }

    #[test]
    fn keeps_code_block_lines() {
        let raw = "Guide\n```rust\nfn main() {\n    println!(\"ok\");\n}\n```\nFooter";
        let cleaned = clean_document_text(raw);
        assert!(cleaned.contains("fn main()"));
        assert!(cleaned.contains("println!"));
    }

    #[test]
    fn drops_toc_like_numbered_lines() {
        let raw = "1.1. Overview\n1.2. Setup\nDeep technical explanation about kernel scheduler internals and process preemption.";
        let cleaned = clean_document_text(raw);
        assert!(!cleaned.contains("1.1. Overview"));
        assert!(cleaned.contains("scheduler internals"));
    }
}
