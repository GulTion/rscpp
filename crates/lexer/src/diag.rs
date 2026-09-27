//! Format lex/parse/runtime errors with line:col and a source caret.

use crate::span::Span;

/// Build a multi-line diagnostic: stage + message, `-->` line:col, source line, caret.
pub fn format_diagnostic(source: &str, span: Span, stage: &str, message: &str) -> String {
    let start = span.start.min(source.len());
    let end = span.end.min(source.len()).max(start);
    let (line, col) = line_col(source, start);
    let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = source[start..]
        .find('\n')
        .map(|i| start + i)
        .unwrap_or(source.len());
    let line_text = &source[line_start..line_end];
    let caret_col = start.saturating_sub(line_start);
    let caret_len = (end - start).max(1).min(line_text.len().saturating_sub(caret_col).max(1));

    let mut out = String::new();
    out.push_str(stage);
    out.push_str(" error: ");
    out.push_str(message);
    out.push('\n');
    out.push_str(&format!("  --> {line}:{col}\n"));
    out.push_str("   |\n");
    out.push_str(&format!(" {line:>2} | {line_text}\n"));
    out.push_str("   | ");
    out.push_str(&" ".repeat(caret_col));
    out.push_str(&"^".repeat(caret_len));
    out.push('\n');
    out
}

/// 1-based line and column for a byte offset.
pub fn line_col(source: &str, byte: usize) -> (usize, usize) {
    let byte = byte.min(source.len());
    let line = source[..byte].bytes().filter(|&b| b == b'\n').count() + 1;
    let col = byte - source[..byte].rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_on_bracket() {
        let src = "int main() {\n  int a[]={1};\n}\n";
        let start = src.find('[').unwrap();
        let s = format_diagnostic(src, Span::new(start, start + 1), "parse", "expected ';', found '['");
        assert!(s.contains("--> 2:8"), "{s}");
        assert!(s.contains("int a[]={1};"), "{s}");
        assert!(s.contains('^'), "{s}");
        assert!(s.contains("expected ';', found '['"), "{s}");
    }
}
