mod definitions;

use std::path::Path;

pub use definitions::{LanguageDefinition, languages};

/// Classifies a path and optional first line into a known language definition.
#[must_use]
pub fn classify(path: &Path, first_line: Option<&str>) -> Option<&'static LanguageDefinition> {
    classify_path(path).or_else(|| {
        first_line
            .and_then(shebang_program)
            .and_then(definitions::by_script)
    })
}

/// Classifies a path using only filename and extension metadata.
#[must_use]
pub fn classify_path(path: &Path) -> Option<&'static LanguageDefinition> {
    let file_name = path.file_name()?.to_string_lossy();
    if let Some(language) = definitions::by_filename(&file_name) {
        return Some(language);
    }

    for ext in ExtensionCandidates::new(file_name.as_ref()) {
        if let Some(language) = definitions::by_extension(ext) {
            return Some(language);
        }
    }

    None
}

/// Classifies a buffer by its shebang line.
#[must_use]
pub fn classify_shebang(bytes: &[u8]) -> Option<&'static LanguageDefinition> {
    let line_end = memchr::memchr(b'\n', bytes).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..line_end])
        .ok()
        .and_then(shebang_program)
        .and_then(definitions::by_script)
}

/// Returns the most specific extension candidate for a path.
#[must_use]
pub fn extension(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();
    ExtensionCandidates::new(&file_name)
        .last()
        .map(str::to_string)
}

struct ExtensionCandidates<'a> {
    file_name: &'a str,
    starts: [usize; 3],
    len: usize,
    index: usize,
}

impl<'a> ExtensionCandidates<'a> {
    fn new(file_name: &'a str) -> Self {
        let mut starts = [0; 3];
        let mut len = 0;
        for (idx, _) in file_name.match_indices('.').take(3) {
            starts[len] = idx + 1;
            len += 1;
        }

        Self {
            file_name,
            starts,
            len,
            index: 0,
        }
    }
}

impl<'a> Iterator for ExtensionCandidates<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }

        let start = self.starts[self.index];
        self.index += 1;
        Some(&self.file_name[start..])
    }
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
