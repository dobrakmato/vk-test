use std::path::{Path, PathBuf};

pub struct Content {
    roots: Vec<Path>,
}

impl Content {
    /// Adds a root the the content system. If the path is link to folder it will
    /// be used as normal file system. If the path points to BF archive, the archive
    /// will be loaded as VFS (virtual file system).
    pub fn add_root(&mut self, root: Path) {
        // todo: vfs
        self.roots.push(root)
    }

    /// Resolves specified path in string to PathBuf that can be used in other operations.
    fn find_file(&mut self, path: &str) -> Option<PathBuf> {
        for p in self.roots.iter() {
            let relative = p.join(Path::new(path));
            if relative.exists() {
                return Some(relative);
            }
        }
        None
    }
}
