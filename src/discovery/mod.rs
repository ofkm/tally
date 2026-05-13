use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ignore::{DirEntry, WalkBuilder};
use regex::Regex;

use crate::language;
use crate::types::{CountError, CountOptions};

const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".jj",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    ".next",
    ".nuxt",
    ".turbo",
    ".cache",
];

#[derive(Debug, Clone)]
/// Files discovered for counting plus diagnostics for ignored paths.
pub struct DiscoveredFiles {
    /// Files accepted by the discovery filters.
    pub files: Vec<PathBuf>,
    /// Human-readable diagnostics for ignored paths.
    pub ignored: Vec<String>,
}

/// Discovers source files from input paths using the provided count options.
///
/// # Errors
///
/// Returns [`CountError`] when an input is missing, a regular expression is
/// invalid, or walking an input directory fails.
pub fn discover(inputs: &[PathBuf], options: &CountOptions) -> Result<DiscoveredFiles, CountError> {
    let filters = DiscoveryFilters::new(options)?;
    let mut files = Vec::new();
    let mut ignored = Vec::new();

    for input in inputs {
        if !input.exists() {
            return Err(CountError::MissingInput(input.clone()));
        }

        if input.is_file() {
            if filters.accept_file(input) {
                files.push(input.clone());
            }
            continue;
        }

        if is_default_ignored_dir(input) || filters.is_excluded_dir(input) {
            ignored.push(format!("ignored directory {}", input.display()));
            continue;
        }

        let mut builder = WalkBuilder::new(input);
        builder
            .hidden(false)
            .follow_links(options.follow_links)
            .filter_entry({
                let filters = filters.clone();
                move |entry| filters.accept_entry(entry)
            });

        for result in builder.build() {
            match result {
                Ok(entry)
                    if entry
                        .file_type()
                        .is_some_and(|file_type| file_type.is_file()) =>
                {
                    let path = entry.path();
                    if filters.accept_file(path) {
                        files.push(path.to_path_buf());
                    }
                }
                Ok(_) => {}
                Err(err) => return Err(CountError::Walk(err.to_string())),
            }
        }
    }

    files.sort();
    files.dedup();

    if !options.skip_uniqueness {
        files = remove_duplicate_files(files, &mut ignored);
    }

    Ok(DiscoveredFiles { files, ignored })
}

fn remove_duplicate_files(files: Vec<PathBuf>, ignored: &mut Vec<String>) -> Vec<PathBuf> {
    let mut by_hash: HashMap<String, PathBuf> = HashMap::new();
    let mut unique = Vec::new();

    for file in files {
        match std::fs::read(&file) {
            Ok(bytes) => {
                let hash = blake3::hash(&bytes).to_hex().to_string();
                if let Some(original) = by_hash.get(&hash) {
                    ignored.push(format!(
                        "{} duplicate of {}",
                        file.display(),
                        original.display()
                    ));
                } else {
                    by_hash.insert(hash, file.clone());
                    unique.push(file);
                }
            }
            Err(_) => unique.push(file),
        }
    }

    unique
}

#[derive(Clone)]
struct DiscoveryFilters {
    include_ext: HashSet<String>,
    exclude_ext: HashSet<String>,
    exclude_dirs: HashSet<String>,
    match_f: Option<Regex>,
    not_match_f: Vec<Regex>,
    match_d: Option<Regex>,
    not_match_d: Vec<Regex>,
    max_file_size_bytes: u64,
}

impl DiscoveryFilters {
    fn new(options: &CountOptions) -> Result<Self, CountError> {
        let mut exclude_dirs = DEFAULT_IGNORED_DIRS
            .iter()
            .map(|dir| (*dir).to_string())
            .collect::<HashSet<_>>();
        exclude_dirs.extend(options.exclude_dir.iter().cloned());

        Ok(Self {
            include_ext: options.include_ext.iter().cloned().collect(),
            exclude_ext: options.exclude_ext.iter().cloned().collect(),
            exclude_dirs,
            match_f: compile_optional_regex(options.match_f.as_deref())?,
            not_match_f: compile_regex_list(&options.not_match_f)?,
            match_d: compile_optional_regex(options.match_d.as_deref())?,
            not_match_d: compile_regex_list(&options.not_match_d)?,
            max_file_size_bytes: max_file_size_bytes(options.max_file_size_mb),
        })
    }

    fn accept_entry(&self, entry: &DirEntry) -> bool {
        let path = entry.path();
        if entry
            .file_type()
            .is_some_and(|file_type| file_type.is_dir())
        {
            !self.is_excluded_dir(path) && self.accept_dir_matchers(path)
        } else {
            true
        }
    }

    fn accept_file(&self, path: &Path) -> bool {
        if path.components().any(|component| {
            let component = component.as_os_str().to_string_lossy();
            self.exclude_dirs.contains(component.as_ref())
        }) {
            return false;
        }

        if !self.accept_dir_matchers(path.parent().unwrap_or_else(|| Path::new(""))) {
            return false;
        }

        let path_text = path.to_string_lossy();
        if let Some(match_f) = &self.match_f
            && !match_f.is_match(&path_text)
        {
            return false;
        }
        if self
            .not_match_f
            .iter()
            .any(|not_match_f| not_match_f.is_match(&path_text))
        {
            return false;
        }

        if let Some(ext) = language::extension(path) {
            let ext = ext.to_ascii_lowercase();
            if !self.include_ext.is_empty() && !self.include_ext.contains(&ext) {
                return false;
            }
            if self.exclude_ext.contains(&ext) {
                return false;
            }
        } else if !self.include_ext.is_empty() {
            return false;
        }

        if self.max_file_size_bytes > 0
            && let Ok(metadata) = path.metadata()
            && metadata.len() > self.max_file_size_bytes
        {
            return false;
        }

        true
    }

    fn is_excluded_dir(&self, path: &Path) -> bool {
        path.file_name()
            .is_some_and(|name| self.exclude_dirs.contains(name.to_string_lossy().as_ref()))
    }

    fn accept_dir_matchers(&self, path: &Path) -> bool {
        let path_text = path.to_string_lossy();
        if let Some(match_d) = &self.match_d
            && !match_d.is_match(&path_text)
        {
            return false;
        }
        !self
            .not_match_d
            .iter()
            .any(|not_match_d| not_match_d.is_match(&path_text))
    }
}

fn max_file_size_bytes(max_file_size_mb: f64) -> u64 {
    if !max_file_size_mb.is_finite() || max_file_size_mb <= 0.0 {
        return 0;
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive finite megabyte values are intentionally rounded down to whole bytes"
    )]
    {
        (max_file_size_mb * 1024.0 * 1024.0) as u64
    }
}

fn compile_optional_regex(pattern: Option<&str>) -> Result<Option<Regex>, CountError> {
    pattern
        .map(|pattern| {
            Regex::new(pattern).map_err(|source| CountError::InvalidRegex {
                pattern: pattern.to_string(),
                source,
            })
        })
        .transpose()
}

fn compile_regex_list(patterns: &[String]) -> Result<Vec<Regex>, CountError> {
    patterns
        .iter()
        .map(|pattern| {
            Regex::new(pattern).map_err(|source| CountError::InvalidRegex {
                pattern: pattern.clone(),
                source,
            })
        })
        .collect()
}

fn is_default_ignored_dir(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        DEFAULT_IGNORED_DIRS
            .iter()
            .any(|ignored| name.to_string_lossy() == *ignored)
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn ignores_default_directories_at_any_depth() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("src/node_modules/pkg")).unwrap();
        fs::create_dir_all(dir.path().join("src/app")).unwrap();
        fs::write(dir.path().join(".git/config"), "x").unwrap();
        fs::write(dir.path().join("src/node_modules/pkg/index.js"), "x").unwrap();
        fs::write(dir.path().join("src/app/main.rs"), "fn main() {}\n").unwrap();

        let result = discover(&[dir.path().to_path_buf()], &CountOptions::default()).unwrap();
        assert_eq!(result.files, vec![dir.path().join("src/app/main.rs")]);
    }

    #[test]
    fn exclude_dir_composes_with_default_ignores() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("vendor")).unwrap();
        fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("vendor/lib.rs"), "fn main() {}\n").unwrap();
        fs::write(dir.path().join("node_modules/lib.rs"), "fn main() {}\n").unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();

        let options = CountOptions {
            exclude_dir: vec!["vendor".to_string()],
            ..CountOptions::default()
        };
        let result = discover(&[dir.path().to_path_buf()], &options).unwrap();
        assert_eq!(result.files, vec![dir.path().join("main.rs")]);
    }
}
