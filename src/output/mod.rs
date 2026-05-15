use std::fmt::Write as _;

use crate::types::Report;

/// Formats a compact text report.
#[must_use]
pub fn format_report(report: &Report) -> String {
    let mut rows = report
        .languages
        .iter()
        .map(|(language, totals)| OutputRow {
            language: language.as_str(),
            files: format_count(totals.files),
            blank: format_count(totals.blank),
            comment: format_count(totals.comment),
            code: format_count(totals.code),
        })
        .collect::<Vec<_>>();
    rows.push(OutputRow {
        language: "TOTAL",
        files: format_count(report.sum.files),
        blank: format_count(report.sum.blank),
        comment: format_count(report.sum.comment),
        code: format_count(report.sum.code),
    });

    let widths = OutputWidths::from_rows(&rows);
    let divider = widths.divider();
    let mut out = String::new();
    let _ = writeln!(out, "Code: {}\n", format_count(report.sum.code));
    let _ = writeln!(
        out,
        "{:<language$}  {:>files$}  {:>blank$}  {:>comment$}  {:>code$}",
        "Language",
        "Files",
        "Blank",
        "Comment",
        "Code",
        language = widths.language,
        files = widths.files,
        blank = widths.blank,
        comment = widths.comment,
        code = widths.code,
    );
    out.push_str(&divider);
    out.push('\n');

    for row in rows {
        if row.language == "TOTAL" {
            out.push_str(&divider);
            out.push('\n');
        }

        let language = color_language(row.language, widths.language);
        let _ = writeln!(
            out,
            "{}  {:>files$}  {:>blank$}  {:>comment$}  {:>code$}",
            language,
            row.files,
            row.blank,
            row.comment,
            row.code,
            files = widths.files,
            blank = widths.blank,
            comment = widths.comment,
            code = widths.code,
        );
    }

    if !report.errors.is_empty() {
        out.push_str("\nErrors:\n");
        for error in &report.errors {
            out.push_str(error);
            out.push('\n');
        }
    }

    out
}

struct OutputRow<'a> {
    language: &'a str,
    files: String,
    blank: String,
    comment: String,
    code: String,
}

#[derive(Clone, Copy)]
struct OutputWidths {
    language: usize,
    files: usize,
    blank: usize,
    comment: usize,
    code: usize,
}

impl OutputWidths {
    fn from_rows(rows: &[OutputRow<'_>]) -> Self {
        rows.iter().fold(
            Self {
                language: "Language".len(),
                files: "Files".len(),
                blank: "Blank".len(),
                comment: "Comment".len(),
                code: "Code".len(),
            },
            |mut widths, row| {
                widths.language = widths.language.max(row.language.len());
                widths.files = widths.files.max(row.files.len());
                widths.blank = widths.blank.max(row.blank.len());
                widths.comment = widths.comment.max(row.comment.len());
                widths.code = widths.code.max(row.code.len());
                widths
            },
        )
    }

    fn divider(self) -> String {
        [
            "-".repeat(self.language),
            "-".repeat(self.files),
            "-".repeat(self.blank),
            "-".repeat(self.comment),
            "-".repeat(self.code),
        ]
        .join("  ")
    }
}

fn format_count(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    grouped.chars().rev().collect()
}

fn color_language(language: &str, width: usize) -> String {
    let padding = " ".repeat(width.saturating_sub(language.len()));
    if language == "TOTAL" {
        return format!("\x1b[1m{language}\x1b[0m{padding}");
    }

    let Color { r, g, b } = language_color(language);
    format!("\x1b[38;2;{r};{g};{b}m{language}\x1b[0m{padding}")
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b }
}

fn language_color(language: &str) -> Color {
    match language {
        "Assembly" => rgb(110, 76, 19),
        "Bazel" | "Bicep" => rgb(74, 179, 79),
        "Blade" => rgb(244, 83, 89),
        "C" => rgb(85, 85, 170),
        "C#" => rgb(98, 39, 116),
        "C++" | "C/C++ Header" => rgb(243, 75, 125),
        "CSS" => rgb(86, 61, 124),
        "CSV" => rgb(60, 160, 60),
        "Dockerfile" => rgb(56, 147, 202),
        "Go" => rgb(0, 173, 216),
        "HTML" | "HTML EEx" => rgb(227, 76, 38),
        "HCL" => rgb(132, 89, 168),
        "INI" => rgb(211, 206, 114),
        "Java" => rgb(176, 114, 25),
        "JavaScript" => rgb(212, 190, 48),
        "JSON" => rgb(160, 160, 160),
        "Justfile" | "make" => rgb(66, 120, 25),
        "Kotlin" => rgb(169, 123, 255),
        "Lua" => rgb(0, 0, 128),
        "Markdown" => rgb(80, 130, 210),
        "Perl" => rgb(2, 152, 195),
        "PHP" => rgb(119, 123, 180),
        "Python" => rgb(53, 114, 165),
        "Ruby" => rgb(204, 52, 45),
        "Rust" => rgb(222, 165, 132),
        "Shell" => rgb(137, 224, 81),
        "SQL" => rgb(218, 94, 41),
        "Svelte" => rgb(255, 62, 0),
        "Swift" => rgb(240, 81, 56),
        "TOML" => rgb(156, 66, 33),
        "TypeScript" => rgb(49, 120, 198),
        "YAML" => rgb(203, 23, 30),
        "Haskell" => rgb(94, 80, 134),
        "XML" => rgb(0, 128, 128),
        _ => fallback_language_color(language),
    }
}

fn fallback_language_color(language: &str) -> Color {
    const PALETTE: &[Color] = &[
        rgb(66, 153, 225),
        rgb(56, 178, 172),
        rgb(72, 187, 120),
        rgb(159, 122, 234),
        rgb(245, 101, 101),
        rgb(236, 201, 75),
        rgb(49, 151, 149),
        rgb(183, 148, 244),
    ];

    let index = language.bytes().fold(0usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as usize)
    }) % PALETTE.len();
    PALETTE[index]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LanguageTotals;

    #[test]
    fn report_starts_with_total_code() {
        let report = Report {
            languages: vec![(
                "Rust".to_string(),
                LanguageTotals {
                    files: 2,
                    blank: 1,
                    comment: 3,
                    code: 1_234,
                },
            )],
            sum: LanguageTotals {
                files: 2,
                blank: 1,
                comment: 3,
                code: 1_234,
            },
            errors: Vec::new(),
        };

        let text = format_report(&report);

        assert!(text.starts_with("Code: 1,234\n\n"));
        assert!(text.contains("Language  Files  Blank  Comment   Code\n"));
        assert!(text.contains("--------  -----  -----  -------  -----\n"));
        assert!(text.contains("Rust"));
        assert!(text.contains("\x1b[38;2;222;165;132mRust\x1b[0m"));
        assert!(text.contains("TOTAL"));
    }
}
