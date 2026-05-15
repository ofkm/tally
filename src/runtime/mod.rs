use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::counter::count_file;
use crate::discovery::discover;
use crate::types::{
    CountError, CountRequest, DirectoryTotals, FileTotals, LanguageTotals, LanguageTree, Report,
};

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
    let tree = if request.tree {
        aggregate_tree(&files, &languages, &request.inputs)
    } else {
        Vec::new()
    };

    Ok(Report {
        languages,
        tree,
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

fn aggregate_tree(
    files: &[FileTotals],
    languages: &[(String, LanguageTotals)],
    inputs: &[PathBuf],
) -> Vec<LanguageTree> {
    let mut by_language = BTreeMap::<&str, BTreeMap<PathBuf, LanguageTotals>>::new();

    for file in files {
        let totals = file.as_language_totals();
        let language_dirs = by_language.entry(&file.language).or_default();
        for directory in directory_ancestors(&relative_directory(&file.path, inputs)) {
            language_dirs.entry(directory).or_default().add(&totals);
        }
    }

    languages
        .iter()
        .filter_map(|(language, _)| {
            let directories = by_language.remove(language.as_str())?;
            Some(LanguageTree {
                language: language.clone(),
                directories: sorted_directories(directories),
            })
        })
        .collect()
}

fn directory_ancestors(directory: &Path) -> BTreeSet<PathBuf> {
    let directory = normalize_directory(directory);
    let mut ancestors = BTreeSet::new();
    ancestors.insert(PathBuf::from("."));

    let mut current = PathBuf::new();
    for component in directory.components() {
        current.push(component.as_os_str());
        if current != Path::new(".") {
            ancestors.insert(current.clone());
        }
    }

    ancestors
}

fn relative_directory(path: &Path, inputs: &[PathBuf]) -> PathBuf {
    if inputs.iter().any(|input| input.is_file() && input == path) {
        return PathBuf::from(".");
    }

    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let relative = inputs
        .iter()
        .filter(|input| input.is_dir())
        .filter_map(|input| directory.strip_prefix(input).ok())
        .min_by_key(|path| path.components().count())
        .unwrap_or(directory);

    normalize_directory(relative)
}

fn normalize_directory(directory: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in directory.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => normalized.push(part),
            _ => normalized.push(component.as_os_str()),
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

fn sorted_directories(directories: BTreeMap<PathBuf, LanguageTotals>) -> Vec<DirectoryTotals> {
    let mut directories = directories
        .into_iter()
        .map(|(path, totals)| DirectoryTotals { path, totals })
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        match (is_root_path(&left.path), is_root_path(&right.path)) {
            (true, true) | (false, false) => left.path.cmp(&right.path),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
        }
    });
    directories
}

fn is_root_path(path: &Path) -> bool {
    path == Path::new(".")
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

    #[test]
    fn tree_aggregation_rolls_counts_up_to_parent_directories() {
        let files = vec![
            FileTotals {
                path: "src/main.rs".into(),
                language: "Rust".to_string(),
                blank: 1,
                comment: 0,
                code: 3,
            },
            FileTotals {
                path: "src/bin/tally.rs".into(),
                language: "Rust".to_string(),
                blank: 0,
                comment: 1,
                code: 2,
            },
            FileTotals {
                path: "scripts/check.py".into(),
                language: "Python".to_string(),
                blank: 0,
                comment: 0,
                code: 5,
            },
        ];
        let (languages, _) = aggregate(&files);

        let tree = aggregate_tree(&files, &languages, &[PathBuf::from(".")]);

        let rust = tree
            .iter()
            .find(|tree| tree.language == "Rust")
            .expect("Rust tree should be present");
        let root = rust
            .directories
            .iter()
            .find(|directory| directory.path == Path::new("."))
            .expect("root directory should be present");
        let src = rust
            .directories
            .iter()
            .find(|directory| directory.path == Path::new("src"))
            .expect("src directory should be present");
        let bin = rust
            .directories
            .iter()
            .find(|directory| directory.path == Path::new("src/bin"))
            .expect("src/bin directory should be present");

        assert_eq!(root.totals.code, 5);
        assert_eq!(src.totals.code, 5);
        assert_eq!(bin.totals.files, 1);
    }
}
