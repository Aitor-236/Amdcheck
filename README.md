# mdcheck

A minimal markdown link checker written in Rust.

> Check broken links and images inside markdown files.

## Feature

- Validate local file & image existence.
- Detect invalid directory targets for image links.

## How to use

Run `mdcheck` with a Markdown file or directory to scan:

```bash
cargo run -- README.md
cargo run -- docs/
```

If no path is provided, the current directory is scanned:

```bash
cargo run
```

The command prints broken links and images | followed by a summary. It exits
with status `1` when broken links are found, which makes it suitable for use
in CI.
