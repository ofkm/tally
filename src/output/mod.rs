use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, presets::ASCII_FULL_CONDENSED};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::types::{
    CountError, DirectoryTotals, FileTotals, LanguageTotals, LanguageTree, OutputFormat, Report,
    ReportHeader,
};

/// Formats a report using the selected output format.
///
/// # Errors
///
/// Returns [`CountError::Output`] if JSON or YAML serialization fails.
pub fn format_report(report: &Report, format: OutputFormat) -> Result<String, CountError> {
    match format {
        OutputFormat::Table => Ok(table(report)),
        OutputFormat::Json => serde_json::to_string_pretty(&MachineReport::from(report))
            .map(|text| format!("{text}\n"))
            .map_err(|err| CountError::Output(err.to_string())),
        OutputFormat::Yaml => serde_yaml_ng::to_string(&MachineReport::from(report))
            .map_err(|err| CountError::Output(err.to_string())),
    }
}

fn table(report: &Report) -> String {
    let mut out = String::new();

    let _ = writeln!(
        out,
        "Files: {}  Lines: {}  Languages: {}  Time: {:.2}s",
        format_count(report.sum.files),
        format_count(report.sum.lines()),
        report.languages.len(),
        report.header.elapsed_seconds
    );

    let denominator = report.sum.lines();
    let mut table = Table::new();
    table.load_preset(ASCII_FULL_CONDENSED).set_header(vec![
        header_cell("Language"),
        right_header_cell("Files"),
        right_header_cell("Blank"),
        right_header_cell("Comment"),
        right_header_cell("Code"),
        right_header_cell("Total"),
        right_header_cell("Share"),
    ]);
    for (language, totals) in &report.languages {
        table.add_row(language_row(language, totals, denominator));
    }

    out.push_str(&table.to_string());
    out.push_str("\n\n");
    out.push_str(&summary_table(&report.sum, denominator));
    out.push('\n');

    if !report.files.is_empty() {
        out.push_str("\nBy file:\n");
        for file in &report.files {
            let _ = writeln!(
                out,
                "{}\t{}\tblank={}\tcomment={}\tcode={}",
                file.path.display(),
                file.language,
                file.blank,
                file.comment,
                file.code
            );
        }
    }

    if !report.tree.is_empty() {
        out.push_str("\nTree:\n");
        for tree in &report.tree {
            out.push_str(&language_tree(tree));
        }
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

fn language_row(language: &str, totals: &LanguageTotals, denominator: u64) -> Vec<Cell> {
    vec![
        language_cell(language),
        right_cell(format_count(totals.files)),
        right_cell(format_count(totals.blank)),
        right_cell(format_count(totals.comment)),
        right_cell(format_count(totals.code)),
        right_cell(format_count(totals.lines())),
        right_cell(format_percent(totals.lines(), denominator)),
    ]
}

fn language_cell(language: &str) -> Cell {
    Cell::new(language)
        .fg(language_color(language))
        .add_attribute(Attribute::Bold)
}

fn right_cell(content: impl Into<String>) -> Cell {
    Cell::new(content.into()).set_alignment(CellAlignment::Right)
}

fn header_cell(content: impl Into<String>) -> Cell {
    Cell::new(content.into()).add_attribute(Attribute::Bold)
}

fn right_header_cell(content: impl Into<String>) -> Cell {
    header_cell(content).set_alignment(CellAlignment::Right)
}

fn summary_table(sum: &LanguageTotals, denominator: u64) -> String {
    let mut table = Table::new();
    table.load_preset(ASCII_FULL_CONDENSED).set_header(vec![
        header_cell("Summary"),
        right_header_cell("Files"),
        right_header_cell("Blank"),
        right_header_cell("Comment"),
        right_header_cell("Code"),
        right_header_cell("Total"),
        right_header_cell("Share"),
    ]);
    table.add_row(vec![
        header_cell("SUM"),
        summary_cell(format_count(sum.files)),
        summary_cell(format_count(sum.blank)),
        summary_cell(format_count(sum.comment)),
        summary_cell(format_count(sum.code)),
        summary_cell(format_count(sum.lines())),
        summary_cell(format_percent(sum.lines(), denominator)),
    ]);
    table.to_string()
}

fn summary_cell(content: impl Into<String>) -> Cell {
    right_cell(content).add_attribute(Attribute::Bold)
}

fn language_tree(tree: &LanguageTree) -> String {
    let denominator = tree
        .directories
        .first()
        .map_or(0, |directory| directory.totals.lines());
    let rows = tree
        .directories
        .iter()
        .map(|directory| {
            let label = tree_directory_label(&directory.path, &tree.directories);
            TreeRow {
                label,
                files: format_count(directory.totals.files),
                blank: format_count(directory.totals.blank),
                comment: format_count(directory.totals.comment),
                code: format_count(directory.totals.code),
                total: format_count(directory.totals.lines()),
                share: format_percent(directory.totals.lines(), denominator),
            }
        })
        .collect::<Vec<_>>();
    let widths = TreeWidths::from_rows(&rows);

    let mut out = String::new();
    let language_files = tree
        .directories
        .first()
        .map_or(0, |directory| directory.totals.files);
    let _ = writeln!(
        out,
        "{}  {}  {}",
        color_text(&tree.language, language_color(&tree.language)),
        format_unit(language_files, "file"),
        format_unit(denominator, "line")
    );
    write_tree_header(&mut out, widths);
    for row in rows {
        write_tree_row(&mut out, &row, widths);
    }
    out.push('\n');
    out
}

#[derive(Debug)]
struct TreeRow {
    label: String,
    files: String,
    blank: String,
    comment: String,
    code: String,
    total: String,
    share: String,
}

#[derive(Debug, Clone, Copy)]
struct TreeWidths {
    label: usize,
    files: usize,
    blank: usize,
    comment: usize,
    code: usize,
    total: usize,
    share: usize,
}

impl TreeWidths {
    fn from_rows(rows: &[TreeRow]) -> Self {
        let mut widths = Self {
            label: "Directory".len(),
            files: "Files".len(),
            blank: "Blank".len(),
            comment: "Comment".len(),
            code: "Code".len(),
            total: "Total".len(),
            share: "Share".len(),
        };

        for row in rows {
            widths.label = widths.label.max(display_width(&row.label));
            widths.files = widths.files.max(row.files.len());
            widths.blank = widths.blank.max(row.blank.len());
            widths.comment = widths.comment.max(row.comment.len());
            widths.code = widths.code.max(row.code.len());
            widths.total = widths.total.max(row.total.len());
            widths.share = widths.share.max(row.share.len());
        }

        widths
    }
}

fn write_tree_header(out: &mut String, widths: TreeWidths) {
    let _ = writeln!(
        out,
        "{}  {:>files$}  {:>blank$}  {:>comment$}  {:>code$}  {:>total$}  {:>share$}",
        pad_right("Directory", widths.label),
        "Files",
        "Blank",
        "Comment",
        "Code",
        "Total",
        "Share",
        files = widths.files,
        blank = widths.blank,
        comment = widths.comment,
        code = widths.code,
        total = widths.total,
        share = widths.share
    );
}

fn write_tree_row(out: &mut String, row: &TreeRow, widths: TreeWidths) {
    let _ = writeln!(
        out,
        "{}  {:>files$}  {:>blank$}  {:>comment$}  {:>code$}  {:>total$}  {:>share$}",
        pad_right(&row.label, widths.label),
        row.files,
        row.blank,
        row.comment,
        row.code,
        row.total,
        row.share,
        files = widths.files,
        blank = widths.blank,
        comment = widths.comment,
        code = widths.code,
        total = widths.total,
        share = widths.share
    );
}

fn tree_directory_label(path: &Path, directories: &[DirectoryTotals]) -> String {
    if path == Path::new(".") {
        return ".".to_string();
    }

    let known_paths = directories
        .iter()
        .map(|directory| directory.path.as_path())
        .collect::<BTreeSet<_>>();
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut label = String::new();

    for depth in 0..components.len().saturating_sub(1) {
        let mut ancestor = PathBuf::new();
        for component in &components[..=depth] {
            ancestor.push(component);
        }
        if has_later_sibling(&ancestor, &known_paths) {
            label.push_str("│  ");
        } else {
            label.push_str("   ");
        }
    }

    if has_later_sibling(path, &known_paths) {
        label.push_str("├─ ");
    } else {
        label.push_str("└─ ");
    }
    if let Some(name) = components.last() {
        label.push_str(name);
    }
    label
}

fn has_later_sibling(path: &Path, known_paths: &BTreeSet<&Path>) -> bool {
    let parent = tree_parent(path);
    known_paths.iter().any(|candidate| {
        *candidate != path && tree_parent(candidate) == parent && *candidate > path
    })
}

fn tree_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn display_width(value: &str) -> usize {
    value.chars().count()
}

fn pad_right(value: &str, width: usize) -> String {
    let padding = width.saturating_sub(display_width(value));
    format!("{value}{}", " ".repeat(padding))
}

fn color_text(value: &str, color: Color) -> String {
    let Some(sequence) = ansi_color_sequence(color) else {
        return value.to_string();
    };
    format!("{sequence}{value}\x1b[0m")
}

fn format_unit(value: u64, unit: &str) -> String {
    let suffix = if value == 1 { "" } else { "s" };
    format!("{} {unit}{suffix}", format_count(value))
}

fn ansi_color_sequence(color: Color) -> Option<String> {
    match color {
        Color::Reset => None,
        Color::Black => Some("\x1b[30m".to_string()),
        Color::DarkGrey => Some("\x1b[90m".to_string()),
        Color::Red => Some("\x1b[91m".to_string()),
        Color::DarkRed => Some("\x1b[31m".to_string()),
        Color::Green => Some("\x1b[92m".to_string()),
        Color::DarkGreen => Some("\x1b[32m".to_string()),
        Color::Yellow => Some("\x1b[93m".to_string()),
        Color::DarkYellow => Some("\x1b[33m".to_string()),
        Color::Blue => Some("\x1b[94m".to_string()),
        Color::DarkBlue => Some("\x1b[34m".to_string()),
        Color::Magenta => Some("\x1b[95m".to_string()),
        Color::DarkMagenta => Some("\x1b[35m".to_string()),
        Color::Cyan => Some("\x1b[96m".to_string()),
        Color::DarkCyan => Some("\x1b[36m".to_string()),
        Color::White => Some("\x1b[97m".to_string()),
        Color::Grey => Some("\x1b[37m".to_string()),
        Color::Rgb { r, g, b } => Some(format!("\x1b[38;2;{r};{g};{b}m")),
        Color::AnsiValue(value) => Some(format!("\x1b[38;5;{value}m")),
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

fn language_color(language: &str) -> Color {
    match language {
        "Assembly" => Color::Rgb {
            r: 110,
            g: 76,
            b: 19,
        },
        "Bazel" | "Bicep" => Color::Rgb {
            r: 74,
            g: 179,
            b: 79,
        },
        "Blade" => Color::Rgb {
            r: 244,
            g: 83,
            b: 89,
        },
        "C" => Color::Rgb {
            r: 85,
            g: 85,
            b: 170,
        },
        "C#" => Color::Rgb {
            r: 98,
            g: 39,
            b: 116,
        },
        "C++" | "C/C++ Header" => Color::Rgb {
            r: 243,
            g: 75,
            b: 125,
        },
        "CSS" => Color::Rgb {
            r: 86,
            g: 61,
            b: 124,
        },
        "CSV" => Color::Rgb {
            r: 60,
            g: 160,
            b: 60,
        },
        "Dockerfile" => Color::Rgb {
            r: 56,
            g: 147,
            b: 202,
        },
        "Go" => Color::Rgb {
            r: 0,
            g: 173,
            b: 216,
        },
        "HTML" | "HTML EEx" => Color::Rgb {
            r: 227,
            g: 76,
            b: 38,
        },
        "HCL" => Color::Rgb {
            r: 132,
            g: 89,
            b: 168,
        },
        "INI" => Color::Rgb {
            r: 211,
            g: 206,
            b: 114,
        },
        "Java" => Color::Rgb {
            r: 176,
            g: 114,
            b: 25,
        },
        "JavaScript" => Color::Rgb {
            r: 212,
            g: 190,
            b: 48,
        },
        "JSON" => Color::Rgb {
            r: 160,
            g: 160,
            b: 160,
        },
        "Justfile" | "make" => Color::Rgb {
            r: 66,
            g: 120,
            b: 25,
        },
        "Kotlin" => Color::Rgb {
            r: 169,
            g: 123,
            b: 255,
        },
        "Lua" => Color::Rgb { r: 0, g: 0, b: 128 },
        "Markdown" => Color::Rgb {
            r: 80,
            g: 130,
            b: 210,
        },
        "Perl" => Color::Rgb {
            r: 2,
            g: 152,
            b: 195,
        },
        "PHP" => Color::Rgb {
            r: 119,
            g: 123,
            b: 180,
        },
        "Python" => Color::Rgb {
            r: 53,
            g: 114,
            b: 165,
        },
        "Ruby" => Color::Rgb {
            r: 204,
            g: 52,
            b: 45,
        },
        "Rust" => Color::Rgb {
            r: 222,
            g: 165,
            b: 132,
        },
        "Shell" => Color::Rgb {
            r: 137,
            g: 224,
            b: 81,
        },
        "SQL" => Color::Rgb {
            r: 218,
            g: 94,
            b: 41,
        },
        "Svelte" => Color::Rgb {
            r: 255,
            g: 62,
            b: 0,
        },
        "Swift" => Color::Rgb {
            r: 240,
            g: 81,
            b: 56,
        },
        "TOML" => Color::Rgb {
            r: 156,
            g: 66,
            b: 33,
        },
        "TypeScript" => Color::Rgb {
            r: 49,
            g: 120,
            b: 198,
        },
        "YAML" => Color::Rgb {
            r: 203,
            g: 23,
            b: 30,
        },
        "Haskell" => Color::Rgb {
            r: 94,
            g: 80,
            b: 134,
        },
        "XML" => Color::Rgb {
            r: 0,
            g: 128,
            b: 128,
        },
        _ => fallback_language_color(language),
    }
}

fn fallback_language_color(language: &str) -> Color {
    const PALETTE: &[Color] = &[
        Color::Blue,
        Color::Cyan,
        Color::Green,
        Color::Magenta,
        Color::Red,
        Color::Yellow,
        Color::DarkCyan,
        Color::DarkMagenta,
    ];

    let index = language.bytes().fold(0usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as usize)
    }) % PALETTE.len();
    PALETTE[index]
}

fn format_percent(value: u64, denominator: u64) -> String {
    if denominator == 0 {
        return "0.0%".to_string();
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "percentage display intentionally rounds large counters to one decimal place"
    )]
    {
        format!("{:.1}%", (value as f64 / denominator as f64) * 100.0)
    }
}

#[derive(Debug, Serialize)]
struct MachineReport<'a> {
    header: &'a ReportHeader,
    #[serde(flatten)]
    languages: BTreeMap<&'a str, &'a LanguageTotals>,
    #[serde(rename = "SUM")]
    sum: &'a LanguageTotals,
    #[serde(skip_serializing_if = "slice_is_empty")]
    files: &'a [FileTotals],
    #[serde(skip_serializing_if = "slice_is_empty")]
    tree: &'a [LanguageTree],
    #[serde(skip_serializing_if = "slice_is_empty")]
    ignored: &'a [String],
    #[serde(skip_serializing_if = "slice_is_empty")]
    errors: &'a [String],
}

impl<'a> From<&'a Report> for MachineReport<'a> {
    fn from(report: &'a Report) -> Self {
        Self {
            header: &report.header,
            languages: report
                .languages
                .iter()
                .map(|(language, totals)| (language.as_str(), totals))
                .collect(),
            sum: &report.sum,
            files: &report.files,
            tree: &report.tree,
            ignored: &report.ignored,
            errors: &report.errors,
        }
    }
}

const fn slice_is_empty<T>(values: &[T]) -> bool {
    values.is_empty()
}
