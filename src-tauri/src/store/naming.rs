use std::path::{Path, PathBuf};

/// Replaces characters that are illegal in Windows/POSIX filenames, so a
/// user-chosen entity name can be used as a file/directory name.
pub fn sanitize_file_stem(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed
    }
}

/// Builds a path under `parent` that doesn't collide with anything already
/// there, appending " (2)", " (3)"… as needed. Without this, creating a
/// second request/folder/environment with a name already in use would
/// silently overwrite the existing file.
pub fn unique_path(parent: &Path, name: &str, extension: &str) -> PathBuf {
    let stem = sanitize_file_stem(name);
    let mut candidate = parent.join(format!("{}{}", stem, extension));
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{} ({}){}", stem, n, extension));
        n += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sanitizes_illegal_characters_and_blank_names() {
        assert_eq!(sanitize_file_stem("a/b:c"), "a_b_c");
        assert_eq!(sanitize_file_stem("   "), "untitled");
        assert_eq!(sanitize_file_stem(" Get Pets "), "Get Pets");
    }

    #[test]
    fn unique_path_avoids_overwriting_existing_entries() {
        let dir = tempfile::tempdir().unwrap();
        let first = unique_path(dir.path(), "List Pets", ".lokki.toml");
        assert_eq!(first.file_name().unwrap(), "List Pets.lokki.toml");
        fs::write(&first, "x").unwrap();

        let second = unique_path(dir.path(), "List Pets", ".lokki.toml");
        assert_eq!(second.file_name().unwrap(), "List Pets (2).lokki.toml");
        fs::write(&second, "x").unwrap();

        let third = unique_path(dir.path(), "List Pets", ".lokki.toml");
        assert_eq!(third.file_name().unwrap(), "List Pets (3).lokki.toml");
    }
}
