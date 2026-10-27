// SPDX-License-Identifier: Apache-2.0

//! Small test-only temporary-directory helper.
//!
//! The workspace avoids a test-only temporary-directory dependency for these
//! bounded tests. Directories are unique per process and test, and are removed
//! on drop.

use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "opentrack-plugin-scan-{}-{id}-{label}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temporary test directory");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write(&self, relative: &str, contents: &[u8]) -> PathBuf {
        let path = self.path.join(portable_relative_path(relative));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent");
        }
        std::fs::write(&path, contents).expect("write fixture file");
        path
    }

    pub fn mkdir(&self, relative: &str) -> PathBuf {
        let path = self.path.join(portable_relative_path(relative));
        std::fs::create_dir_all(&path).expect("create fixture directory");
        path
    }
}

fn portable_relative_path(relative: &str) -> PathBuf {
    relative
        .split(['/', '\\'])
        .filter(|component| !component.is_empty())
        .collect()
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
