//! Recording-owned media lifecycle. Imported/existing audio is never registered here.
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(1);

pub(super) struct RecordingUndo {
    paths: Vec<PathBuf>,
    root: PathBuf,
    cache: Option<PathBuf>,
    hidden: bool,
    retain_on_error: bool,
}
impl RecordingUndo {
    pub(super) fn new(paths: Vec<PathBuf>, root: PathBuf) -> Self {
        Self {
            paths,
            root,
            cache: None,
            hidden: false,
            retain_on_error: false,
        }
    }
    pub(super) fn remove(
        &mut self,
        restored: &opentrack_core::project::Project,
    ) -> Result<(), String> {
        if self.paths.iter().any(|p| {
            restored
                .tracks
                .iter()
                .any(|t| t.clips.iter().any(|c| c.source_path == *p))
        }) {
            return Err("a recorded file is still referenced by the restored project".into());
        }
        if self.cache.is_none() {
            std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
            let dir = loop {
                let dir = self.root.join(format!(
                    "recording-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                match std::fs::create_dir(&dir) {
                    Ok(()) => break dir,
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => return Err(e.to_string()),
                }
            };
            let manifest = self
                .paths
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{i}.wav -> {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n");
            if let Err(error) = std::fs::write(dir.join("paths.txt"), manifest) {
                let _ = std::fs::remove_dir(&dir);
                return Err(error.to_string());
            }
            self.cache = Some(dir);
        }
        if let Err(error) = self.transfer(false) {
            self.retain_on_error = true;
            return Err(error);
        }
        self.retain_on_error = false;
        self.hidden = true;
        Ok(())
    }
    pub(super) fn restore(&mut self) -> Result<(), String> {
        if !self.hidden {
            return Ok(());
        }
        if let Err(error) = self.transfer(true) {
            self.retain_on_error = true;
            return Err(error);
        }
        self.retain_on_error = false;
        self.hidden = false;
        Ok(())
    }
    pub(super) fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
    pub(super) fn protects_saved(&self, saved: &std::collections::HashSet<PathBuf>) -> bool {
        self.hidden && self.paths.iter().any(|p| saved.contains(p))
    }
    fn transfer(&self, restore: bool) -> Result<(), String> {
        let cache = self
            .cache
            .as_ref()
            .ok_or("recording recovery data is unavailable")?;
        let pairs = self
            .paths
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let backup = cache.join(format!("{i}.wav"));
                if restore {
                    (backup, p.clone())
                } else {
                    (p.clone(), backup)
                }
            })
            .collect::<Vec<_>>();
        // Preflight the complete take group before changing any file.
        for (source, dest) in &pairs {
            if !source.is_file() {
                return Err(format!(
                    "recording file is unavailable: {}",
                    source.display()
                ));
            }
            if dest.exists() {
                return Err(format!(
                    "refusing to replace an existing file: {}",
                    dest.display()
                ));
            }
        }
        for (i, (source, dest)) in pairs.iter().enumerate() {
            if let Err(error) = move_no_replace(source, dest) {
                let mut message = format!("{}: {error}", source.display());
                for (source, dest) in pairs[..i].iter().rev() {
                    if let Err(error) = move_no_replace(dest, source) {
                        message.push_str(&format!(
                            "; rollback failed for {}: {error}",
                            source.display()
                        ));
                    }
                }
                return Err(message);
            }
        }
        Ok(())
    }
}
impl Drop for RecordingUndo {
    fn drop(&mut self) {
        if self.retain_on_error {
            return;
        } // leave recovery evidence after a failed/partial filesystem transaction
        if let Some(cache) = &self.cache {
            // Only our numbered recovery files are owned here. Never delete originals.
            for i in 0..self.paths.len() {
                let _ = std::fs::remove_file(cache.join(format!("{i}.wav")));
            }
            let _ = std::fs::remove_file(cache.join("paths.txt"));
            let _ = std::fs::remove_dir(cache);
            let _ = std::fs::remove_dir(&self.root); // only succeeds if empty
        }
    }
}
fn move_no_replace(source: &Path, dest: &Path) -> std::io::Result<()> {
    // Same-volume hard links avoid copying long takes. Neither path can be overwritten.
    if std::fs::hard_link(source, dest).is_err() {
        use std::io::Write;
        let mut input = std::fs::File::open(source)?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)?;
        if let Err(error) = std::io::copy(&mut input, &mut output)
            .and_then(|_| output.flush())
            .and_then(|_| output.sync_all())
        {
            drop(output);
            let _ = std::fs::remove_file(dest);
            return Err(error);
        }
    }
    if let Err(error) = std::fs::remove_file(source) {
        let _ = std::fs::remove_file(dest);
        return Err(error);
    }
    Ok(())
}
