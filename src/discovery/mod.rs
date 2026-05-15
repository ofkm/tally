use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use ignore::{DirEntry, WalkBuilder};

use crate::types::CountError;

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

static DEFAULT_IGNORED_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| DEFAULT_IGNORED_DIRS.iter().copied().collect());

#[derive(Debug, Clone)]
/// Files discovered for counting.
pub struct DiscoveredFiles {
    /// Files accepted by discovery.
    pub files: Vec<PathBuf>,
}

/// Discovers source files from input paths.
///
/// # Errors
///
/// Returns [`CountError`] when an input is missing or walking an input directory
/// fails.
pub fn discover(inputs: &[PathBuf]) -> Result<DiscoveredFiles, CountError> {
    let filters = DiscoveryFilters;
    let mut files = Vec::new();

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

        if filters.is_excluded_dir(input) {
            continue;
        }

        let mut builder = WalkBuilder::new(input);
        builder.hidden(false).filter_entry({
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
                    files.push(path.to_path_buf());
                }
                Ok(_) => {}
                Err(err) => return Err(CountError::Walk(err.to_string())),
            }
        }
    }

    files.sort();
    files.dedup();

    Ok(DiscoveredFiles { files })
}

#[derive(Clone)]
struct DiscoveryFilters;

impl DiscoveryFilters {
    fn accept_entry(&self, entry: &DirEntry) -> bool {
        let path = entry.path();
        !entry
            .file_type()
            .is_some_and(|file_type| file_type.is_dir() && self.is_excluded_dir(path))
    }

    fn accept_file(&self, path: &Path) -> bool {
        !path.components().any(|component| {
            let component = component.as_os_str().to_string_lossy();
            DEFAULT_IGNORED_SET.contains(component.as_ref())
        })
    }

    fn is_excluded_dir(&self, path: &Path) -> bool {
        path.file_name()
            .is_some_and(|name| DEFAULT_IGNORED_SET.contains(name.to_string_lossy().as_ref()))
    }
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

        let result = discover(&[dir.path().to_path_buf()]).unwrap();
        assert_eq!(result.files, vec![dir.path().join("src/app/main.rs")]);
    }
}
