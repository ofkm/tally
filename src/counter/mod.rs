use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use aho_corasick::AhoCorasick;
use memchr::{memchr, memchr2, memmem};

use crate::language::{self, LanguageDefinition};
use crate::types::{CountError, FileTotals};

static LINE_COMMENT_MATCHERS: LazyLock<HashMap<&'static str, AhoCorasick>> = LazyLock::new(|| {
    language::languages()
        .iter()
        .filter(|language| !language.line_comments.is_empty())
        .filter_map(|language| {
            AhoCorasick::new(language.line_comments)
                .ok()
                .map(|matcher| (language.name, matcher))
        })
        .collect()
});

/// Counts one file and returns per-file totals when the language is supported.
///
/// Binary files and files with unknown languages return `Ok(None)`.
///
/// # Errors
///
/// Returns [`CountError::Read`] when the file cannot be read.
pub fn count_file(path: &Path) -> Result<Option<FileTotals>, CountError> {
    let path_language = language::classify_path(path);
    if path_language.is_none() && !could_have_shebang(path) {
        return Ok(None);
    }

    let bytes = std::fs::read(path).map_err(|source| CountError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let Some(language) = path_language.or_else(|| language::classify_shebang(&bytes)) else {
        return Ok(None);
    };

    let Some((blank, comment, code)) = count_source_bytes(&bytes, language) else {
        return Ok(None);
    };
    Ok(Some(FileTotals {
        path: path.to_path_buf(),
        language: language.name.to_string(),
        blank,
        comment,
        code,
    }))
}

fn could_have_shebang(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !name.contains('.'))
}

/// Counts blank, comment, and code lines for text using a language definition.
#[must_use]
pub fn count_text(text: &str, language: &LanguageDefinition) -> (u64, u64, u64) {
    count_bytes(text.as_bytes(), language)
}

/// Counts blank, comment, and code lines for UTF-8 compatible source bytes.
#[must_use]
pub fn count_bytes(bytes: &[u8], language: &LanguageDefinition) -> (u64, u64, u64) {
    count_bytes_inner(bytes, language, false).unwrap_or_default()
}

fn count_source_bytes(bytes: &[u8], language: &LanguageDefinition) -> Option<(u64, u64, u64)> {
    count_bytes_inner(bytes, language, true)
}

fn count_bytes_inner(
    bytes: &[u8],
    language: &LanguageDefinition,
    reject_binary: bool,
) -> Option<(u64, u64, u64)> {
    let mut blank = 0;
    let mut comment = 0;
    let mut code = 0;
    let mut block_stack: Vec<(&[u8], &[u8])> = Vec::new();
    let line_comment_matcher = LINE_COMMENT_MATCHERS.get(language.name);

    let mut line_start = 0;
    while line_start < bytes.len() {
        let line_end = match memchr2(b'\n', 0, &bytes[line_start..]) {
            Some(offset) => {
                let idx = line_start + offset;
                if reject_binary && bytes[idx] == 0 {
                    return None;
                }
                idx
            }
            None => bytes.len(),
        };
        let line = &bytes[line_start..line_end];
        let trimmed = trim_ascii(line);
        if trimmed.is_empty() {
            blank += 1;
        } else {
            let classification =
                classify_line(line, language, &mut block_stack, line_comment_matcher);
            match classification {
                LineClass::Code => code += 1,
                LineClass::Comment => comment += 1,
            }
        }

        if line_end == bytes.len() {
            break;
        }
        line_start = line_end + 1;
    }

    Some((blank, comment, code))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineClass {
    Code,
    Comment,
}

fn classify_line(
    line: &[u8],
    language: &LanguageDefinition,
    block_stack: &mut Vec<(&'static [u8], &'static [u8])>,
    line_comment_matcher: Option<&AhoCorasick>,
) -> LineClass {
    let mut rest = trim_start_ascii(line);
    let mut saw_code = false;

    loop {
        if let Some((_, end)) = block_stack.last().copied() {
            if let Some(end_idx) = find_bytes(rest, end) {
                rest = &rest[end_idx + end.len()..];
                block_stack.pop();
                if trim_ascii(rest).is_empty() {
                    return if saw_code {
                        LineClass::Code
                    } else {
                        LineClass::Comment
                    };
                }
                continue;
            }
            return if saw_code {
                LineClass::Code
            } else {
                LineClass::Comment
            };
        }

        let next_block = earliest_block_start(rest, language.block_comments);
        let next_line = earliest_line_comment(rest, line_comment_matcher);
        match (next_block, next_line) {
            (None, None) => {
                if !trim_ascii(rest).is_empty() {
                    saw_code = true;
                }
                return if saw_code {
                    LineClass::Code
                } else {
                    LineClass::Comment
                };
            }
            (Some((block_idx, _, _)), Some(line_idx)) if line_idx < block_idx => {
                if !trim_ascii(&rest[..line_idx]).is_empty() {
                    saw_code = true;
                }
                return if saw_code {
                    LineClass::Code
                } else {
                    LineClass::Comment
                };
            }
            (None, Some(line_idx)) => {
                if !trim_ascii(&rest[..line_idx]).is_empty() {
                    saw_code = true;
                }
                return if saw_code {
                    LineClass::Code
                } else {
                    LineClass::Comment
                };
            }
            (Some((block_idx, start, end)), _) => {
                if !trim_ascii(&rest[..block_idx]).is_empty() {
                    saw_code = true;
                }
                let after_start = &rest[block_idx + start.len()..];
                if let Some(end_idx) = find_bytes(after_start, end) {
                    rest = &after_start[end_idx + end.len()..];
                    if trim_ascii(rest).is_empty() {
                        return if saw_code {
                            LineClass::Code
                        } else {
                            LineClass::Comment
                        };
                    }
                    continue;
                }
                block_stack.push((start, end));
                return if saw_code {
                    LineClass::Code
                } else {
                    LineClass::Comment
                };
            }
        }
    }
}

fn earliest_block_start(
    text: &[u8],
    blocks: &'static [(&'static str, &'static str)],
) -> Option<(usize, &'static [u8], &'static [u8])> {
    blocks
        .iter()
        .filter_map(|(start, end)| {
            let start = start.as_bytes();
            let end = end.as_bytes();
            find_bytes(text, start).map(|idx| (idx, start, end))
        })
        .min_by_key(|(idx, _, _)| *idx)
}

fn earliest_line_comment(text: &[u8], matcher: Option<&AhoCorasick>) -> Option<usize> {
    matcher.and_then(|matcher| matcher.find(text).map(|hit| hit.start()))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    match needle {
        [] => Some(0),
        [byte] => memchr(*byte, haystack),
        _ => memmem::find(haystack, needle),
    }
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    trim_end_ascii(trim_start_ascii(bytes))
}

fn trim_start_ascii(bytes: &[u8]) -> &[u8] {
    let first = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[first..]
}

fn trim_end_ascii(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(0, |idx| idx + 1);
    &bytes[..end]
}

#[cfg(test)]
mod tests {
    use crate::language::languages;

    use super::*;

    fn lang(name: &str) -> &'static LanguageDefinition {
        languages()
            .iter()
            .find(|language| language.name == name)
            .unwrap()
    }

    #[test]
    fn counts_c_style_comments() {
        let text = "/* head */\n\nint main() {\n  return 0; // ok\n}\n";
        assert_eq!(count_text(text, lang("C")), (1, 1, 3));
    }

    #[test]
    fn counts_python_docstrings_as_comments() {
        let text = "\"\"\"module\"\"\"\n\nprint('x')\n# tail\n";
        assert_eq!(count_text(text, lang("Python")), (1, 2, 1));
    }

    #[test]
    fn counts_html_comments() {
        let text = "<!-- x -->\n<div>\n</div>\n";
        assert_eq!(count_text(text, lang("HTML")), (0, 1, 2));
    }

    #[test]
    fn counts_blade_template_comments() {
        let text = "{{-- x --}}\n<html>\n<!-- y -->\n</html>\n";
        assert_eq!(count_text(text, lang("Blade")), (0, 2, 2));
    }

    #[test]
    fn counts_svelte_html_and_script_comments() {
        let text = "<!-- head -->\n<script>\n/* block */\nconst count = 0;\n// tail\n</script>\n";
        assert_eq!(count_text(text, lang("Svelte")), (0, 3, 3));
    }
}
