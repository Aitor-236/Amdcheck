use regex::Regex;
use std::sync::LazyLock;

static LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(!?)\[([^\]]*)\]\(([^)]+)\)").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    Link,
    Image,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MarkdownLink {
    pub destination: String,
    pub line: usize,
    pub kind: LinkKind,
}

/// Replace fence code block (``` ... ```) and inline code(`...`) with space
/// Preserve line breaks and byte length,
/// keep line and column numbers aligned with the original source text.
fn strip_code(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;

    while i < bytes.len() {
        // 1. Try match start of fence code block
        // allow 0~3 leading spaces at the start of line
        if is_fence_start(bytes, i) {
            let fence_len = fence_length(&bytes[i..]);
            let body_start = i + fence_len;
            if let Some(end) = find_fence_end(bytes, body_start, fence_len) {
                blank_out(&mut out, body_start, end);
                i = end;
                continue;
            }
        }

        // 2.inline code
        if bytes[i] == b'`' {
            let tick_len = run_length(bytes, i, b'`');
            if let Some(end) = find_inline_end(bytes, i + tick_len, tick_len) {
                blank_out(&mut out, i + tick_len, end);
                i = end + tick_len;
                continue;
            }
            i += tick_len;
            continue;
        }
        i += 1;
    }
    String::from_utf8(out).unwrap()
}

/// Check if position 'i' is the start of a fenced code block
/// (at line start, or after 0~3 leading spaces followed by triple backticks ```)
fn is_fence_start(bytes: &[u8], i: usize) -> bool {
    if i >= bytes.len() {
        return false;
    }
    // Must be at the beginning of a line
    if i > 0 && bytes[i - 1] != b'\n' {
        return false;
    }

    // Allow 0 to 3 leading spaces of indentation
    let mut j = i;
    let mut spaces = 0;
    while j < bytes.len() && bytes[j] == b' ' && spaces < 4 {
        j += 1;
        spaces += 1;
    }
    spaces <= 3 && j + 2 < bytes.len() && &bytes[j..j + 3] == b"```"
}

/// Starting at index 'i'(pointing to the first backtick of ```)
/// return the count of consecutive fence backticks
fn fence_length(bytes: &[u8]) -> usize {
    run_length(bytes, 0, b'`')
}

/// Count consecutive occurrences of byte 'c', starting from index 'i'
fn run_length(bytes: &[u8], i: usize, c: u8) -> usize {
    let mut n = 0;
    while i + n < bytes.len() && bytes[i + n] == c {
        n += 1;
    }
    n
}

fn find_fence_end(bytes: &[u8], body_start: usize, fence_len: usize) -> Option<usize> {
    let mut i = body_start;
    while i < bytes.len() {
        // find next indentation
        if bytes[i] == b'\n' {
            let line_start = i + 1;
            let mut j = line_start;
            let mut spaces = 0;
            while j < bytes.len() && bytes[j] == b' ' && spaces < 4 {
                j += 1;
                spaces += 1;
            }
            if spaces <= 3
                && j + fence_len <= bytes.len()
                && run_length(bytes, j, b'`') >= fence_len
            {
                return Some(line_start);
            }
        }
        i += 1;
    }
    None
}

fn find_inline_end(bytes: &[u8], from: usize, tick_len: usize) -> Option<usize> {
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let n = run_length(bytes, i, b'`');
            if n == tick_len {
                return Some(i);
            }
            i += n;
        } else {
            i += 1;
        }
    }
    None
}

/// change non-space char in out[start..end] to space.
fn blank_out(out: &mut [u8], start: usize, end: usize) {
    for b in &mut out[start..end] {
        if *b != b'\n' {
            *b = b' ';
        }
    }
}

pub fn extract_links(content: &str) -> Vec<MarkdownLink> {
    let stripped = strip_code(content);
    let content = stripped.as_str();

    let mut links: Vec<MarkdownLink> = Vec::new();

    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(content.match_indices('\n').map(|(i, _)| i + 1))
        .collect();

    let line_of = |pos: usize| -> usize {
        match line_starts.binary_search(&pos) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    };

    for cap in LINK_RE.captures_iter(&content) {
        let m = cap.get(0).unwrap();
        let line = line_of(m.start());
        let kind = if &cap[1] == "!" {
            LinkKind::Image
        } else {
            LinkKind::Link
        };
        let url = &cap[3];

        links.push(MarkdownLink {
            destination: url.to_string(),
            line,
            kind,
        });
    }
    links
}

#[cfg(test)]
mod extract_links_tests {
    use super::*;

    #[test]
    fn extract_single_link() {
        let content = "this is a [Guide](./guide.md).";

        let links = extract_links(content);

        let expected = vec![MarkdownLink {
            destination: "./guide.md".to_string(),
            line: 1,
            kind: LinkKind::Link,
        }];

        assert_eq!(links, expected);
    }

    #[test]
    fn extract_multiple_link() {
        let content = "this is a [Guide](./guide.md).\nREADME tell us how to use the tool.\nGo to [README](./README.md)";

        let links = extract_links(content);

        let expected = vec![
            MarkdownLink {
                destination: "./guide.md".to_string(),
                line: 1,
                kind: LinkKind::Link,
            },
            MarkdownLink {
                destination: "./README.md".to_string(),
                line: 3,
                kind: LinkKind::Link,
            },
        ];

        assert_eq!(links, expected);
    }

    #[test]
    fn extract_links_and_image() {
        let content = "this is a[Guide](./guide.md).\nREADME tell us how to use the tool![smile](./smile.png).\nGo to [README](./README.md).";

        let links = extract_links(content);

        let expected = vec![
            MarkdownLink {
                destination: "./guide.md".to_string(),
                line: 1,
                kind: LinkKind::Link,
            },
            MarkdownLink {
                destination: "./smile.png".to_string(),
                line: 2,
                kind: LinkKind::Image,
            },
            MarkdownLink {
                destination: "./README.md".to_string(),
                line: 3,
                kind: LinkKind::Link,
            },
        ];

        assert_eq!(links, expected);
    }

    #[test]
    fn extract_image_only() {
        let content = "![alt](./a.png)";
        let links = extract_links(content);
        assert_eq!(
            links,
            vec![MarkdownLink {
                destination: "./a.png".to_string(),
                line: 1,
                kind: LinkKind::Image,
            }]
        );
    }

    #[test]
    fn no_links() {
        assert!(extract_links("plain text, no links").is_empty());
    }
    #[test]
    fn link_at_first_line_after_newline() {
        // 验证 pos 正好等于行首时 Ok 分支
        let content = "\n[x](u)";
        let links = extract_links(content);
        assert_eq!(links[0].line, 2);
    }

    #[test]
    fn url_with_title_truncation_behavior() {
        // 显式记录当前局限，防止未来回归时无感知
        let content = r#"[x](https://a.com "title")"#;
        let links = extract_links(content);
        assert_eq!(links[0].destination, r#"https://a.com "title""#);
    }

    #[test]
    fn empty_content() {
        assert!(extract_links("").is_empty());
    }
    #[test]
    fn ignore_links_in_fenced_code_block() {
        let content = "```\n[x](fake.md)\n```\nreal [y](real.md)";
        let links = extract_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].destination, "real.md");
        assert_eq!(links[0].line, 4);
    }

    #[test]
    fn ignore_links_in_inline_code() {
        let content = "use `[x](fake.md)` and [y](real.md)";
        let links = extract_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].destination, "real.md");
    }

    #[test]
    fn code_block_preserves_line_numbers() {
        let content = "```\nline2\nline3\n```\n[a](b.md)";
        let links = extract_links(content);
        assert_eq!(links[0].line, 5);
    }

    #[test]
    fn unclosed_fence_ignored_gracefully() {
        // 未闭合的 fence：当前实现会找结束 fence 失败，回退到按 inline code 处理，
        // 这里仅确保不 panic、行号不乱
        let content = "```\n[x](a.md)";
        let _ = extract_links(content);
    }

    #[test]
    fn inline_code_with_link_inside_fence() {
        let content = "```\n`[a](b.md)`\n```\n[c](d.md)";
        let links = extract_links(content);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].destination, "d.md");
    }
}

#[cfg(test)]
mod strip_code_tests {
    use super::*;

    /// Helper: after stripping code blocks, non-empty content should match expectation
    fn stripped(content: &str) -> String {
        strip_code(content)
    }

    #[test]
    fn plain_text_unchanged() {
        let s = "hello [a](b.md)\nworld";
        assert_eq!(stripped(s), s);
    }
}

#[cfg(test)]
mod is_fence_start_tests {
    use super::*;

    /// Helper: get first "```" position
    fn get_fence_index(input: &[u8]) -> usize {
        input.windows(3).position(|w| w == b"```").unwrap()
    }

    #[test]
    fn test_basic_fence_at_start() {
        let input = b"```rust\nlet x = 1;\n```";
        assert!(is_fence_start(input, 0));
    }

    #[test]
    fn test_fence_after_newline() {
        let input = b"some text\n```python\nprint(1)\n```";
        assert!(is_fence_start(input, get_fence_index(input)));
    }

    #[test]
    fn test_not_at_line_start() {
        let input = b"some text```java\nint a = 1;\n```";
        assert!(!is_fence_start(input, get_fence_index(input)));
    }

    #[test]
    fn test_tab_indentation_not_treated_as_space() {
        assert!(!is_fence_start(b"\t```", 0));
    }

    #[test]
    fn test_valid_indentation_1_to_3_spaces() {
        // 1 indentation
        assert!(is_fence_start(b" ```", 0));
        // 2 indentation
        assert!(is_fence_start(b"  ```", 0));
        // 3 indentation
        assert!(is_fence_start(b"   ```", 0));
    }

    #[test]
    fn test_too_many_spaces_4_or_more() {
        // 4 indentation
        assert!(!is_fence_start(b"    ```", 0));
        // 5 indentation
        assert!(!is_fence_start(b"     ```", 0));
    }

    #[test]
    fn test_fewer_than_three_backticks() {
        assert!(!is_fence_start(b"`", 0));
        assert!(!is_fence_start(b"``", 0));
        assert!(!is_fence_start(b" ``", 0));
    }

    #[test]
    fn test_more_than_three_backticks() {
        assert!(is_fence_start(b"````", 0));
        assert!(is_fence_start(b" `````rust", 0));
    }

    #[test]
    fn test_empty_and_out_of_bounds_index() {
        assert!(!is_fence_start(b"", 0));
        assert!(!is_fence_start(b"```", 3));
        assert!(!is_fence_start(b"```", 1000));
    }
}

#[cfg(test)]
mod run_length_tests {
    use super::*;

    #[test]
    fn test_basic_count_length() {
        assert_eq!(run_length(b"aaaaa3fds", 0, b'a'), 5);
        assert_eq!(run_length(b"aaaaa3fds", 1, b'a'), 4);
    }

    #[test]
    fn test_non_satiate_char() {
        assert_eq!(run_length(b"just do it", 0, b'a'), 0);
    }

    #[test]
    fn test_empty_and_out_of_bounds_index() {
        assert_eq!(run_length(b"", 0, b'a'), 0);
        assert_eq!(run_length(b"aaa", 3, b'a'), 0);
    }
}
