use std::collections::BTreeMap;

use rayon::prelude::*;

use crate::counter::count_file;
use crate::discovery::discover;
use crate::types::{CountError, CountRequest, FileTotals, LanguageTotals, Report};

/// Counts source files for a request and returns a report.
///
/// # Errors
///
/// Returns [`CountError`] when input discovery fails. File-level read errors are
/// captured inside the report so one unreadable file does not stop the count.
pub fn count(request: &CountRequest) -> Result<Report, CountError> {
    let discovered = discover(&request.inputs)?;
    let counted = discovered
        .files
        .par_iter()
        .map(|path| count_file(path))
        .collect::<Vec<_>>();

    let mut files = Vec::new();
    let mut errors = Vec::new();
    for result in counted {
        match result {
            Ok(Some(file)) => files.push(file),
            Ok(None) => {}
            Err(err) => errors.push(err.to_string()),
        }
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    let (languages, sum) = aggregate(&files);

    Ok(Report {
        languages,
        sum,
        errors,
    })
}

fn aggregate(files: &[FileTotals]) -> (Vec<(String, LanguageTotals)>, LanguageTotals) {
    let mut by_language = BTreeMap::<String, LanguageTotals>::new();
    let mut sum = LanguageTotals::default();

    for file in files {
        let totals = file.as_language_totals();
        by_language
            .entry(file.language.clone())
            .or_default()
            .add(&totals);
        sum.add(&totals);
    }

    let mut languages = by_language.into_iter().collect::<Vec<_>>();
    languages.sort_by(|(left_name, left), (right_name, right)| {
        right
            .code
            .cmp(&left.code)
            .then_with(|| right.comment.cmp(&left.comment))
            .then_with(|| right.blank.cmp(&left.blank))
            .then_with(|| left_name.cmp(right_name))
    });
    (languages, sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregation_is_deterministic() {
        let files = vec![
            FileTotals {
                path: "b.rs".into(),
                language: "Rust".to_string(),
                blank: 1,
                comment: 1,
                code: 2,
            },
            FileTotals {
                path: "a.py".into(),
                language: "Python".to_string(),
                blank: 1,
                comment: 0,
                code: 2,
            },
        ];

        let (languages, sum) = aggregate(&files);
        assert_eq!(sum.files, 2);
        assert_eq!(sum.code, 4);
        assert_eq!(languages[0].0, "Rust");
        assert_eq!(languages[1].0, "Python");
    }
}
