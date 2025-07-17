pub fn categorize_diff(diff: &str) -> Vec<String> {
    let mut categories = vec![];

    let lines: Vec<&str> = diff.lines().collect();

    let mut added_lines = lines.iter().filter(|l| l.starts_with('+')).map(|l| *l).collect::<Vec<_>>();

    if added_lines.iter().any(|line| line.contains("fn") && line.contains("test"))
        || added_lines.iter().any(|line| line.contains("#[test]"))
    {
        categories.push("test".to_string());
    }

    if added_lines.iter().any(|line| line.contains("fix") || line.contains("bug") || line.contains("unwrap()")) {
        categories.push("fix".to_string());
    }

    if added_lines.iter().any(|line| line.contains("///") || line.contains("//") || line.contains("doc")) {
        categories.push("docs".to_string());
    }

    if added_lines.iter().any(|line| line.contains("fn ") || line.contains("impl ") || line.contains("struct ")) {
        categories.push("feat".to_string());
    }

    if added_lines.iter().any(|line| line.trim_start_matches('+').trim().is_empty()) {
        categories.push("style".to_string());
    }

    if added_lines.iter().any(|line| line.contains("let ") || line.contains("const ") || line.contains("=")) {
        categories.push("refactor".to_string());
    }

    // If no category was confidently matched, fallback to 'chore'
    if categories.is_empty() {
        categories.push("chore".to_string());
    }

    categories.sort();
    categories.dedup();
    categories
}
