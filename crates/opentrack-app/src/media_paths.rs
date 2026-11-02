//! Filesystem containment for project-relative media destinations.
//!
//! A pool folder is validated lexically by `opentrack-core`, but an existing
//! junction or symlink inside the project can still redirect a seemingly safe
//! relative folder outside it. Every writer resolves the destination through
//! this module before creating it, so the project never writes outside its own
//! tree through a link.

use std::path::{Path, PathBuf};

/// Resolves `root/subfolder` after proving the resolved path stays inside
/// `root`, creating the folder only after the existing components are checked.
///
/// The returned path keeps the caller's spelling of `root` so stored project
/// paths stay readable; canonicalization is used only for the containment
/// check. A missing tail is allowed: once an existing component is known to be
/// inside the project, the remaining directories are new and cannot redirect.
pub(crate) fn project_subfolder_directory(root: &Path, subfolder: &str) -> Result<PathBuf, String> {
    let canonical_root = std::fs::canonicalize(root).map_err(|error| {
        format!(
            "Could not resolve the project folder {}: {error}",
            root.display()
        )
    })?;
    let mut probe = root.to_path_buf();
    for part in subfolder.split(['/', '\\']) {
        if part.is_empty() {
            return Err("The media folder has an empty path component".to_owned());
        }
        probe.push(part);
        match std::fs::symlink_metadata(&probe) {
            Ok(_) => {
                let canonical = std::fs::canonicalize(&probe).map_err(|error| {
                    format!(
                        "Could not resolve the media folder {}: {error}",
                        probe.display()
                    )
                })?;
                if !canonical.starts_with(&canonical_root) {
                    return Err(format!(
                        "The media folder {} leaves the project folder through a link or junction",
                        probe.display()
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(format!(
                    "Could not inspect the media folder {}: {error}",
                    probe.display()
                ));
            }
        }
    }
    let directory = root.join(subfolder);
    std::fs::create_dir_all(&directory).map_err(|error| {
        format!(
            "Could not create the media folder {}: {error}",
            directory.display()
        )
    })?;
    // Re-check after creation: a link created between the preflight and here
    // must not turn the destination into an escape.
    let canonical_directory = std::fs::canonicalize(&directory).map_err(|error| {
        format!(
            "Could not resolve the media folder {}: {error}",
            directory.display()
        )
    })?;
    if !canonical_directory.starts_with(&canonical_root) {
        return Err(format!(
            "The media folder {} leaves the project folder through a link or junction",
            directory.display()
        ));
    }
    Ok(directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "opentrack-media-paths-{name}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn a_normal_subfolder_is_created_inside_the_project() {
        let root = scratch("normal");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("root");
        let directory = project_subfolder_directory(&root, "Audio/Imports").expect("destination");
        assert!(directory.exists());
        assert!(directory.starts_with(&root));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_out_of_the_project_is_refused_and_creates_nothing() {
        let root = scratch("junction");
        let outside = scratch("junction-outside");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&root).expect("root");
        fs::create_dir_all(&outside).expect("outside");
        let link = root.join("Audio");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .status();
        let Ok(status) = status else {
            let _ = fs::remove_dir_all(&root);
            let _ = fs::remove_dir_all(&outside);
            return;
        };
        if !status.success() {
            let _ = fs::remove_dir_all(&root);
            let _ = fs::remove_dir_all(&outside);
            return;
        }
        let error = project_subfolder_directory(&root, "Audio/Imports")
            .expect_err("junction escape must be refused");
        assert!(error.contains("leaves the project folder"));
        assert!(
            !outside.join("Imports").exists(),
            "the refused destination must not be created outside the project"
        );
        let _ = fs::remove_dir(&link);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_the_project_is_refused_and_creates_nothing() {
        use std::os::unix::fs::symlink;

        let root = scratch("symlink");
        let outside = scratch("symlink-outside");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&root).expect("root");
        fs::create_dir_all(&outside).expect("outside");
        symlink(&outside, root.join("Audio")).expect("symlink");
        let error = project_subfolder_directory(&root, "Audio/Imports")
            .expect_err("symlink escape must be refused");
        assert!(error.contains("leaves the project folder"));
        assert!(!outside.join("Imports").exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }
}
