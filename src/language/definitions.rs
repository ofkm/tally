use std::collections::HashMap;
use std::sync::LazyLock;

/// Syntax metadata used to classify and count a language.
#[derive(Debug, Clone, Copy)]
pub struct LanguageDefinition {
    /// Display name used in reports.
    pub name: &'static str,
    /// Single-line comment delimiters.
    pub line_comments: &'static [&'static str],
    /// Block comment start/end delimiters.
    pub block_comments: &'static [(&'static str, &'static str)],
    /// Whether block comments can nest.
    pub nested_block_comments: bool,
}

impl LanguageDefinition {
    const fn new(
        name: &'static str,
        line_comments: &'static [&'static str],
        block_comments: &'static [(&'static str, &'static str)],
    ) -> Self {
        Self {
            name,
            line_comments,
            block_comments,
            nested_block_comments: false,
        }
    }

    const fn nested(
        name: &'static str,
        line_comments: &'static [&'static str],
        block_comments: &'static [(&'static str, &'static str)],
    ) -> Self {
        Self {
            name,
            line_comments,
            block_comments,
            nested_block_comments: true,
        }
    }
}

const SLASH: &[&str] = &["//"];
const HASH: &[&str] = &["#"];
const DASH: &[&str] = &["--"];
const XML_BLOCK: &[(&str, &str)] = &[("<!--", "-->")];
const C_BLOCK: &[(&str, &str)] = &[("/*", "*/")];
const PY_BLOCK: &[(&str, &str)] = &[(r#"""""#, r#"""""#), ("'''", "'''")];
const LUA_BLOCK: &[(&str, &str)] = &[("--[[", "]]")];
const RUBY_BLOCK: &[(&str, &str)] = &[("=begin", "=end")];
const BLADE_BLOCK: &[(&str, &str)] = &[("{{--", "--}}"), ("<!--", "-->")];
const HTML_BLOCK: &[(&str, &str)] = &[("<!--", "-->")];
const SVELTE_BLOCK: &[(&str, &str)] = &[("<!--", "-->"), ("/*", "*/")];
const NONE_LINES: &[&str] = &[];
const NONE_BLOCKS: &[(&str, &str)] = &[];

static LANGUAGES: LazyLock<Vec<LanguageDefinition>> = LazyLock::new(|| {
    vec![
        LanguageDefinition::new("Assembly", &[";", "#"], C_BLOCK),
        LanguageDefinition::new("Bazel", HASH, NONE_BLOCKS),
        LanguageDefinition::new("Bicep", SLASH, C_BLOCK),
        LanguageDefinition::new("Blade", NONE_LINES, BLADE_BLOCK),
        LanguageDefinition::new("C", SLASH, C_BLOCK),
        LanguageDefinition::new("C#", SLASH, C_BLOCK),
        LanguageDefinition::new("C++", SLASH, C_BLOCK),
        LanguageDefinition::new("C/C++ Header", SLASH, C_BLOCK),
        LanguageDefinition::new("CSS", NONE_LINES, C_BLOCK),
        LanguageDefinition::new("CSV", NONE_LINES, NONE_BLOCKS),
        LanguageDefinition::new("Dockerfile", HASH, NONE_BLOCKS),
        LanguageDefinition::new("Go", SLASH, C_BLOCK),
        LanguageDefinition::new("HTML", NONE_LINES, HTML_BLOCK),
        LanguageDefinition::new("HTML EEx", NONE_LINES, &[("<!--", "-->"), ("<%#", "%>")]),
        LanguageDefinition::new("HCL", HASH, C_BLOCK),
        LanguageDefinition::new("INI", &["#", ";"], NONE_BLOCKS),
        LanguageDefinition::new("Java", SLASH, C_BLOCK),
        LanguageDefinition::new("JavaScript", SLASH, C_BLOCK),
        LanguageDefinition::new("JSON", NONE_LINES, NONE_BLOCKS),
        LanguageDefinition::new("Justfile", HASH, NONE_BLOCKS),
        LanguageDefinition::new("Kotlin", SLASH, C_BLOCK),
        LanguageDefinition::new("Lua", DASH, LUA_BLOCK),
        LanguageDefinition::new("make", HASH, NONE_BLOCKS),
        LanguageDefinition::new("Markdown", NONE_LINES, NONE_BLOCKS),
        LanguageDefinition::new("Perl", HASH, &[("=pod", "=cut")]),
        LanguageDefinition::new("PHP", SLASH, C_BLOCK),
        LanguageDefinition::new("Python", HASH, PY_BLOCK),
        LanguageDefinition::new("Ruby", HASH, RUBY_BLOCK),
        LanguageDefinition::new("Rust", SLASH, C_BLOCK),
        LanguageDefinition::new("Shell", HASH, NONE_BLOCKS),
        LanguageDefinition::new("SQL", DASH, C_BLOCK),
        LanguageDefinition::new("Svelte", SLASH, SVELTE_BLOCK),
        LanguageDefinition::new("Swift", SLASH, C_BLOCK),
        LanguageDefinition::new("TOML", HASH, NONE_BLOCKS),
        LanguageDefinition::new("TypeScript", SLASH, C_BLOCK),
        LanguageDefinition::new("YAML", HASH, NONE_BLOCKS),
        LanguageDefinition::nested("Haskell", DASH, &[("{-", "-}")]),
        LanguageDefinition::new("XML", NONE_LINES, XML_BLOCK),
    ]
});

static BY_NAME: LazyLock<HashMap<&'static str, &'static LanguageDefinition>> =
    LazyLock::new(|| {
        LANGUAGES
            .iter()
            .map(|language| (language.name, language))
            .collect()
    });

static BY_EXTENSION: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("asm", "Assembly"),
        ("s", "Assembly"),
        ("S", "Assembly"),
        ("BUILD", "Bazel"),
        ("bzl", "Bazel"),
        ("bicep", "Bicep"),
        ("blade.php", "Blade"),
        ("c", "C"),
        ("h", "C/C++ Header"),
        ("cc", "C++"),
        ("cpp", "C++"),
        ("cxx", "C++"),
        ("hpp", "C/C++ Header"),
        ("cs", "C#"),
        ("css", "CSS"),
        ("csv", "CSV"),
        ("dockerfile", "Dockerfile"),
        ("go", "Go"),
        ("html", "HTML"),
        ("htm", "HTML"),
        ("heex", "HTML EEx"),
        ("hcl", "HCL"),
        ("tf", "HCL"),
        ("ini", "INI"),
        ("java", "Java"),
        ("js", "JavaScript"),
        ("jsx", "JavaScript"),
        ("mjs", "JavaScript"),
        ("cjs", "JavaScript"),
        ("json", "JSON"),
        ("just", "Justfile"),
        ("kt", "Kotlin"),
        ("kts", "Kotlin"),
        ("lua", "Lua"),
        ("mk", "make"),
        ("md", "Markdown"),
        ("markdown", "Markdown"),
        ("pl", "Perl"),
        ("pm", "Perl"),
        ("php", "PHP"),
        ("py", "Python"),
        ("pyw", "Python"),
        ("rb", "Ruby"),
        ("rs", "Rust"),
        ("sh", "Shell"),
        ("bash", "Shell"),
        ("zsh", "Shell"),
        ("fish", "Shell"),
        ("sql", "SQL"),
        ("svelte", "Svelte"),
        ("swift", "Swift"),
        ("toml", "TOML"),
        ("ts", "TypeScript"),
        ("tsx", "TypeScript"),
        ("yaml", "YAML"),
        ("yml", "YAML"),
        ("hs", "Haskell"),
        ("xml", "XML"),
    ])
});

static BY_FILENAME: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("Dockerfile", "Dockerfile"),
        ("Containerfile", "Dockerfile"),
        ("Makefile", "make"),
        ("makefile", "make"),
        ("GNUMakefile", "make"),
        ("CMakeLists.txt", "make"),
        ("Justfile", "Justfile"),
    ])
});

static BY_SCRIPT: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("bash", "Shell"),
        ("sh", "Shell"),
        ("zsh", "Shell"),
        ("fish", "Shell"),
        ("perl", "Perl"),
        ("python", "Python"),
        ("python2", "Python"),
        ("python3", "Python"),
        ("ruby", "Ruby"),
        ("node", "JavaScript"),
    ])
});

/// Returns all known language definitions.
#[must_use]
pub fn languages() -> &'static [LanguageDefinition] {
    &LANGUAGES
}

/// Looks up a language by file extension.
#[must_use]
pub fn by_extension(extension: &str) -> Option<&'static LanguageDefinition> {
    BY_EXTENSION
        .get(extension)
        .or_else(|| {
            let key = extension.to_ascii_lowercase();
            BY_EXTENSION.get(key.as_str())
        })
        .and_then(|name| BY_NAME.get(name).copied())
}

/// Looks up a language by exact filename.
#[must_use]
pub fn by_filename(file_name: &str) -> Option<&'static LanguageDefinition> {
    BY_FILENAME
        .get(file_name)
        .and_then(|name| BY_NAME.get(name).copied())
}

/// Looks up a language by shebang script name.
#[must_use]
pub fn by_script(script: &str) -> Option<&'static LanguageDefinition> {
    BY_SCRIPT
        .get(script)
        .and_then(|name| BY_NAME.get(name).copied())
}
