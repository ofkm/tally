use std::path::Path;

use aho_corasick::AhoCorasick;
use memchr::memchr;

use crate::language::{self, LanguageDefinition};
use crate::types::{CountError, FileTotals};

/// Counts one file and returns per-file totals when the language is supported.
///
/// Binary files and files with unknown languages return `Ok(None)`.
///
/// # Errors
///
/// Returns [`CountError::Read`] when the file cannot be read.
pub fn count_file(path: &Path) -> Result<Option<FileTotals>, CountError> {
    let bytes = std::fs::read(path).map_err(|source| CountError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if looks_binary(&bytes) {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&bytes);
    let first_line = text.lines().next();
    let Some(language) = language::classify(path, first_line) else {
        return Ok(None);
    };

    let (blank, comment, code) = count_text(&text, language);
    Ok(Some(FileTotals {
        path: path.to_path_buf(),
        language: language.name.to_string(),
        blank,
        comment,
        code,
    }))
}

/// Counts blank, comment, and code lines for text using a language definition.
#[must_use]
pub fn count_text(text: &str, language: &LanguageDefinition) -> (u64, u64, u64) {
    let mut blank = 0;
    let mut comment = 0;
    let mut code = 0;
    let mut block_stack: Vec<(&str, &str)> = Vec::new();
    let line_comment_matcher = AhoCorasick::new(language.line_comments).ok();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank += 1;
            continue;
        }

        let classification = classify_line(
            line,
            language,
            &mut block_stack,
            line_comment_matcher.as_ref(),
        );
        match classification {
            LineClass::Code => code += 1,
            LineClass::Comment => comment += 1,
        }
    }

    (blank, comment, code)
}

fn looks_binary(bytes: &[u8]) -> bool {
    memchr(0, bytes).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineClass {
    Code,
    Comment,
}

fn classify_line(
    line: &str,
    language: &LanguageDefinition,
    block_stack: &mut Vec<(&'static str, &'static str)>,
    line_comment_matcher: Option<&AhoCorasick>,
) -> LineClass {
    let mut rest = line.trim_start();
    let mut saw_code = false;

    loop {
        if let Some((_, end)) = block_stack.last().copied() {
            if let Some(end_idx) = rest.find(end) {
                rest = &rest[end_idx + end.len()..];
                block_stack.pop();
                if rest.trim().is_empty() {
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
                if !rest.trim().is_empty() {
                    saw_code = true;
                }
                return if saw_code {
                    LineClass::Code
                } else {
                    LineClass::Comment
                };
            }
            (Some((block_idx, _, _)), Some(line_idx)) if line_idx < block_idx => {
                if !rest[..line_idx].trim().is_empty() {
                    saw_code = true;
                }
                return if saw_code {
                    LineClass::Code
                } else {
                    LineClass::Comment
                };
            }
            (None, Some(line_idx)) => {
                if !rest[..line_idx].trim().is_empty() {
                    saw_code = true;
                }
                return if saw_code {
                    LineClass::Code
                } else {
                    LineClass::Comment
                };
            }
            (Some((block_idx, start, end)), _) => {
                if !rest[..block_idx].trim().is_empty() {
                    saw_code = true;
                }
                let after_start = &rest[block_idx + start.len()..];
                if let Some(end_idx) = after_start.find(end) {
                    rest = &after_start[end_idx + end.len()..];
                    if rest.trim().is_empty() {
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

fn earliest_block_start<'a>(
    text: &str,
    blocks: &'a [(&'a str, &'a str)],
) -> Option<(usize, &'a str, &'a str)> {
    blocks
        .iter()
        .filter_map(|(start, end)| text.find(start).map(|idx| (idx, *start, *end)))
        .min_by_key(|(idx, _, _)| *idx)
}

fn earliest_line_comment(text: &str, matcher: Option<&AhoCorasick>) -> Option<usize> {
    matcher.and_then(|matcher| matcher.find(text).map(|hit| hit.start()))
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
