use std::path::PathBuf;

use anyhow::{Context, Result};
use glob::{MatchOptions, glob_with};

pub fn expand(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let options = MatchOptions {
        case_sensitive: !cfg!(windows),
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let mut out = Vec::new();
    for input in inputs {
        let s = input.to_string_lossy();
        if !looks_like_glob(&s) {
            out.push(input.clone());
            continue;
        }
        let mut matched = 0usize;
        for entry in glob_with(&s, options).with_context(|| format!("invalid glob pattern: {s}"))? {
            out.push(entry.with_context(|| format!("globbing {s}"))?);
            matched += 1;
        }
        if matched == 0 {
            eprintln!("vimg: no matches for {s}");
        }
    }
    Ok(out)
}

fn looks_like_glob(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}
