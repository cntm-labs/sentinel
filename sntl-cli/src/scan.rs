//! Workspace scanner that locates `sntl::query*!` invocations.
//!
//! v0.2 uses a regex matcher rather than full AST parsing — fast enough for
//! `sntl prepare` and good enough to catch every form the macros accept.
//! Full `syn::parse_file` upgrade is tracked as a v0.3 follow-up.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A discovered invocation of a sntl query macro.
pub struct Discovered {
    pub file: PathBuf,
    pub line: u32,
    pub sql: String,
}

const MACRO_PATTERN: &str = concat!(
    r#"sntl::query(?:_as|_scalar|_file|_file_as|_pipeline|_unchecked|_as_unchecked)?!"#,
    r#"\s*\(\s*(?:[^,\)]*,\s*)?""#,
    r#"(?P<sql>(?:[^"\\]|\\.)*)""#,
);

pub fn scan(root: &Path) -> std::io::Result<Vec<Discovered>> {
    let re = regex::Regex::new(MACRO_PATTERN).expect("static regex compiles");
    let mut out = vec![];
    for entry in WalkDir::new(root).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = match std::fs::read_to_string(entry.path()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for (line_no, line) in text.lines().enumerate() {
            for cap in re.captures_iter(line) {
                out.push(Discovered {
                    file: entry.path().to_path_buf(),
                    line: (line_no as u32) + 1,
                    sql: cap["sql"].to_string(),
                });
            }
        }
    }
    Ok(out)
}
