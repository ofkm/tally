mod definitions;

use std::path::Path;

pub use definitions::{LanguageDefinition, languages};

/// Classifies a path and optional first line into a known language definition.
#[must_use]
pub fn classify(path: &Path, first_line: Option<&str>) -> Option<&'static LanguageDefinition> {
    let file_name = path.file_name()?.to_string_lossy();
    if let Some(language) = definitions::by_filename(&file_name) {
        return Some(language);
    }

    for ext in extension_candidates(&file_name) {
        if let Some(language) = definitions::by_extension(&ext) {
            return Some(language);
        }
    }

    first_line
        .and_then(shebang_program)
        .and_then(definitions::by_script)
}

/// Returns the most specific extension candidate for a path.
#[must_use]
pub fn extension(path: &Path) -> Option<String> {
    path.file_name().and_then(|name| {
        extension_candidates(&name.to_string_lossy())
            .into_iter()
            .last()
    })
}

fn extension_candidates(file_name: &str) -> Vec<String> {
    let parts: Vec<&str> = file_name.split('.').collect();
    if parts.len() < 2 {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let max_parts = parts.len().min(4);
    for start in 1..max_parts {
        candidates.push(parts[start..].join("."));
    }
    candidates
}

fn shebang_program(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("#!")?.trim();
    let mut parts = rest.split_whitespace();
    let command = parts.next()?.rsplit('/').next()?;
    if command == "env" {
        parts.next()
    } else {
        Some(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_multi_part_extensions_before_single_extension() {
        let language = classify(Path::new("master.blade.php"), None).unwrap();
        assert_eq!(language.name, "Blade");
    }

    #[test]
    fn classifies_exact_filenames() {
        let language = classify(Path::new("Dockerfile"), None).unwrap();
        assert_eq!(language.name, "Dockerfile");
    }

    #[test]
    fn classifies_shebang_files() {
        let language = classify(Path::new("script"), Some("#!/usr/bin/env python3")).unwrap();
        assert_eq!(language.name, "Python");
    }

    #[test]
    fn classifies_svelte_files() {
        let language = classify(Path::new("component.svelte"), None).unwrap();
        assert_eq!(language.name, "Svelte");
    }
}
