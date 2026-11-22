use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};
fn collect_from_dir(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    // read dir, get readDir
    let entries = fs::read_dir(path)?;
    for entry in entries {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_from_dir(&p, files)?;
        } else if p.is_file() && is_markdown(&p) {
            files.push(p)
        }
    }
    Ok(())
}
pub fn collect_md_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    let metadata = fs::metadata(path)?;
    let mut files = Vec::new();

    if metadata.is_file() {
        if is_markdown(path) {
            files.push(path.to_path_buf());
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The file is not Markdown!",
            ));
        }
    } else if metadata.is_dir() {
        collect_from_dir(path, &mut files)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The path is not a common file or dir!",
        ));
    }
    files.sort();
    Ok(files)
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    use tempfile::tempdir;

    #[test]
    fn scans_markdown_files_in_single_directory() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        /*
        <tempdir>/
        ├── README.md
        ├── notes.txt
        └── api.md
        */
        fs::write(root.join("README.md"), "# Home").unwrap();
        fs::write(root.join("notes.txt"), "Ignore").unwrap();
        fs::write(root.join("api.md"), "# api").unwrap();
        let files = collect_md_files(root).unwrap();

        assert_eq!(files.len(), 2);

        let expected = vec![root.join("README.md"), root.join("api.md")];

        assert_eq!(files, expected);
    }

    #[test]
    fn scans_markdown_files_recursively() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        /*
        <tempdir>/
        ├── README.md
        ├── guide/
        │   ├── install.markdown
        │   ├── usage.md
        │   └── advanced/
        │       └───config.md
        └── assets/
            └── logo.png
        */
        fs::write(root.join("README.md"), "# Home").unwrap();

        fs::create_dir_all(root.join("guide")).unwrap();
        fs::write(root.join("guide/install.markdown"), "# Install").unwrap();
        fs::write(root.join("guide/usage.md"), "# Usage").unwrap();
        fs::create_dir_all(root.join("guide/advanced")).unwrap();
        fs::write(root.join("guide/advanced/config.md"), "# Config").unwrap();

        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("assets/logo.png"), b"").unwrap();

        let files = collect_md_files(root).unwrap();

        assert_eq!(files.len(), 4);

        let mut expected = vec![
            root.join("README.md"),
            root.join("guide/install.markdown"),
            root.join("guide/usage.md"),
            root.join("guide/advanced/config.md"),
        ];
        expected.sort();

        assert_eq!(files, expected);
    }

    #[test]
    fn handles_empty_directory() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        let files = collect_md_files(root).unwrap();

        assert_eq!(files.len(), 0);

        let expected: Vec<PathBuf> = Vec::new();
        assert_eq!(files, expected);
    }

    #[test]
    fn returns_error_for_nonexistent_path() {
        let path = Path::new("this_directory_does_not_exist");
        let result = collect_md_files(path);
        assert!(result.is_err());
    }

    #[test]
    fn handles_single_markdown_file() {
        let temp = tempdir().unwrap();
        let root = temp.path();

        let file = root.join("README.md");

        fs::write(&file, "# Home").unwrap();

        let files = collect_md_files(&file).unwrap();

        assert_eq!(files.len(), 1);

        let expected = vec![root.join("README.md")];
        assert_eq!(files, expected);
    }

    #[test]
    fn recognizes_markdown_extensions() {
        assert!(is_markdown(Path::new("README.md")));
        assert!(is_markdown(Path::new("guide.markdown")));

        // test case insensiticity
        assert!(is_markdown(Path::new("README.MD")));
        assert!(is_markdown(Path::new("README.Markdown")));
    }

    #[test]
    fn rejects_non_markdown_extensions() {
        assert!(!is_markdown(Path::new("notes.txt")));
        assert!(!is_markdown(Path::new("image.png")));
        assert!(!is_markdown(Path::new("Makefile")));
    }
}
