use comfy_table::{Attribute, Cell, CellAlignment, Color, Table, presets::ASCII_FULL_CONDENSED};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::types::{CountError, FileTotals, LanguageTotals, OutputFormat, Report, ReportHeader};

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
        Cell::new(language).fg(Color::Green),
        right_cell(format_count(totals.files)),
        right_cell(format_count(totals.blank)),
        right_cell(format_count(totals.comment)),
        right_cell(format_count(totals.code)).fg(Color::Yellow),
        right_cell(format_count(totals.lines())).fg(Color::Cyan),
        right_cell(format_percent(totals.lines(), denominator)).fg(Color::Magenta),
    ]
}

fn right_cell(content: impl Into<String>) -> Cell {
    Cell::new(content.into()).set_alignment(CellAlignment::Right)
}

fn header_cell(content: impl Into<String>) -> Cell {
    Cell::new(content.into())
        .fg(Color::Cyan)
        .add_attribute(Attribute::Bold)
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
        header_cell("SUM").fg(Color::Magenta),
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
    right_cell(content)
        .fg(Color::Magenta)
        .add_attribute(Attribute::Bold)
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
            ignored: &report.ignored,
            errors: &report.errors,
        }
    }
}

const fn slice_is_empty<T>(values: &[T]) -> bool {
    values.is_empty()
}
