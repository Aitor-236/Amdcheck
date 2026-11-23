use std::path::{Path, PathBuf};

use std::io::ErrorKind;

use crate::markdown::{LinkKind, MarkdownLink};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    /// Target exists and is valid.
    Ok,
    /// Target is missing or invalid (e.g., file does not exist, path points to a directory)
    Broken { reason: String },
    /// Skipped or does not require filesystem checks (e.g., external URLs, anchors, empty targets).
    Skipped { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub file: PathBuf,
    pub line: usize,
    pub kind: LinkKind,
    pub destination: String,
    pub status: CheckStatus,
}

/// Checks all links within a single Markdown file.
pub fn check_links(
    source_file: &Path,
    links: &[MarkdownLink],
    root_dir: Option<&Path>,
) -> Vec<CheckResult> {
    links
        .iter()
        .map(|link| check_single_link(source_file, link, root_dir))
        .collect()
}

/// Checks a single Markdown link against its source file location.
pub fn check_single_link(
    source_file: &Path,
    link: &MarkdownLink,
    root_dir: Option<&Path>,
) -> CheckResult {
    let status = evaluate_destination(&link.destination, &link.kind, source_file, root_dir);
    CheckResult {
        file: source_file.to_path_buf(),
        line: link.line,
        kind: link.kind.clone(),
        destination: link.destination.clone(),
        status,
    }
}

fn evaluate_destination(
    raw_dest: &str,
    kind: &LinkKind,
    source_file: &Path,
    root_dir: Option<&Path>,
) -> CheckStatus {
    let dest = raw_dest.trim();

    if dest.is_empty() {
        return CheckStatus::Skipped {
            reason: "empty destination".to_string(),
        };
    }

    if dest.starts_with('#') {
        return CheckStatus::Skipped {
            reason: "auchor link".to_string(),
        };
    }

    if is_external_scheme(dest) {
        return CheckStatus::Skipped {
            reason: "external scheme".to_string(),
        };
    }

    // Strip query string (?...) and fragment (#...) before resolving path.
    let path_part = strip_query_and_fragment(dest);
    if path_part.is_empty() {
        return CheckStatus::Skipped {
            reason: "empty path after stripping query/fragment".to_string(),
        };
    }

    let target = if let Some(rel_from_root) = path_part.strip_prefix('/') {
        match root_dir {
            Some(root) => root.join(rel_from_root),
            None => {
                return CheckStatus::Skipped {
                    reason: "root-relative path without root_dir".to_string(),
                };
            }
        }
    } else {
        let base_dir = source_file
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        base_dir.join(path_part)
    };

    match std::fs::metadata(&target) {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return CheckStatus::Broken {
                reason: format!("target not found: {}", target.display()),
            };
        }
        Err(e) => {
            return CheckStatus::Broken {
                reason: format!("cannot access target {}: {e}", target.display()),
            };
        }
    }

    if matches!(kind, LinkKind::Image) && target.is_dir() {
        return CheckStatus::Broken {
            reason: format!("image points to a directory: {}", target.display()),
        };
    }

    CheckStatus::Ok
}

fn strip_query_and_fragment(dest: &str) -> &str {
    let without_query = dest.split('?').next().unwrap_or(dest);
    without_query
        .split('#')
        .next()
        .unwrap_or(without_query)
        .trim()
}

fn is_external_scheme(dest: &str) -> bool {
    let lower = dest.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("data:")
        || lower.starts_with("javascript:")
}

#[cfg(test)]
mod strip_query_and_fragment_tests {
    use super::*;

    #[test]
    fn test_strip_query_and_fragment() {
        assert_eq!(strip_query_and_fragment("a.md?x=1#sec"), "a.md");
        assert_eq!(strip_query_and_fragment("a.md#sec"), "a.md");
        assert_eq!(strip_query_and_fragment("a.md?x=1"), "a.md");
        assert_eq!(strip_query_and_fragment("#sec"), "");
        assert_eq!(strip_query_and_fragment("a.md"), "a.md");
        assert_eq!(strip_query_and_fragment("  a.md  "), "a.md");
    }
}

#[cfg(test)]
mod evaluate_destination_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn relative_link_exists() {
        let dir = tempdir().unwrap();
        let md = dir.path().join("a.md");
        fs::write(&md, "").unwrap();
        fs::write(dir.path().join("b.md"), "").unwrap();

        let status = evaluate_destination("./b.md", &LinkKind::Link, &md, None);
        assert_eq!(status, CheckStatus::Ok);
    }

    #[test]
    fn relative_link_missing() {
        let dir = tempdir().unwrap();
        let md = dir.path().join("a.md");
        fs::write(&md, "").unwrap();

        let status = evaluate_destination("./missing.md", &LinkKind::Link, &md, None);
        assert!(matches!(status, CheckStatus::Broken { .. }));
    }

    #[test]
    fn anchor_skipped() {
        let status = evaluate_destination("#sec", &LinkKind::Link, Path::new("a.md"), None);
        assert!(matches!(status, CheckStatus::Skipped { .. }));
    }

    #[test]
    fn external_skipped() {
        for d in [
            "https://a.com",
            "mailto:x@y.com",
            "tel:123",
            "data:text/plain,x",
        ] {
            let status = evaluate_destination(d, &LinkKind::Link, Path::new("a.md"), None);
            assert!(matches!(status, CheckStatus::Skipped { .. }), "failed: {d}");
        }
    }

    #[test]
    fn image_to_dir_broken() {
        let dir = tempdir().unwrap();
        let md = dir.path().join("a.md");
        fs::write(&md, "").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        let status = evaluate_destination("./sub", &LinkKind::Image, &md, None);
        assert!(matches!(status, CheckStatus::Broken { .. }));
    }

    #[test]
    fn link_to_dir_ok() {
        let dir = tempdir().unwrap();
        let md = dir.path().join("a.md");
        fs::write(&md, "").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        // 普通链接指向目录是合法的（如 [docs](./docs/)）
        let status = evaluate_destination("./sub", &LinkKind::Link, &md, None);
        assert_eq!(status, CheckStatus::Ok);
    }

    #[test]
    fn root_relative_without_root_skipped() {
        let status = evaluate_destination("/docs/a.md", &LinkKind::Link, Path::new("a.md"), None);
        assert!(matches!(status, CheckStatus::Skipped { .. }));
    }

    #[test]
    fn root_relative_with_root() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "").unwrap();

        let status = evaluate_destination(
            "/a.md",
            &LinkKind::Link,
            Path::new("whatever.md"),
            Some(dir.path()),
        );
        assert_eq!(status, CheckStatus::Ok);
    }

    #[test]
    fn query_and_fragment_stripped_before_check() {
        let dir = tempdir().unwrap();
        let md = dir.path().join("a.md");
        fs::write(&md, "").unwrap();
        fs::write(dir.path().join("b.md"), "").unwrap();

        let status = evaluate_destination("./b.md?x=1#sec", &LinkKind::Link, &md, None);
        assert_eq!(status, CheckStatus::Ok);
    }
}
