use std::path::{Path, PathBuf};
use std::fs::{read_to_string, File};

enum Error {
    NotFound,
    IOError(String),
}

pub struct Content {
    roots: Vec<PathBuf>,
}

impl Default for Content {
    fn default() -> Self {
        Content {
            roots: vec![],
        }
    }
}

impl Content {
    pub fn new() -> Self {
        Content::default()
    }

    /// Adds a root the the content system. If the path is link to folder it will
    /// be used as normal file system. If the path points to BF archive, the archive
    /// will be loaded as VFS (virtual file system).
    pub fn add_root(&mut self, root: PathBuf) {
        // todo: vfs
        self.roots.push(root)
    }

    /// Resolves specified path in string to PathBuf that can be used in other operations.
    fn find_file(&self, path: &str) -> Option<PathBuf> {
        for p in self.roots.iter() {
            let relative = p.join(Path::new(path));
            if relative.exists() {
                return Some(relative);
            }
        }
        None
    }

    /// Returns true if file specified by the path exists, false otherwise.
    pub fn exists(&self, path: &str) -> bool {
        return self.find_file(path).is_some();
    }

    /// Loads file specified by path to String or returns Error if file does not
    /// exists or there is other problem with reading the file.
    pub fn load_utf8(&self, path: &str) -> Result<String, Error> {
        let resolved = self.find_file(path)
            .ok_or(Error::NotFound)?;

        read_to_string(resolved)
            .map_err(|e| Error::IOError(e.to_string()))
    }

    /// Loads file specified by path to Vec<u8> or returns Error if file does not
    /// exists or there is other problem with reading the file.
    pub fn load_binary(&self, path: &str) -> Result<File, Error> {
        let resolved = self.find_file(path)
            .ok_or(Error::NotFound)?;

        File::open(resolved)
            .map_err(|e| Error::IOError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::content::Content;
    use std::path::Path;
    use std::io::Read;
    use std::fs::File;

    #[test]
    fn exists() {
        let mut content = Content::default();

        content.add_root(Path::new("resources").to_owned());

        assert!(content.exists("text_file.txt"));
        assert!(!content.exists("non_existing.txt"));
    }

    #[test]
    fn read_utf8() {
        let mut content = Content::default();

        content.add_root(Path::new("resources").to_owned());

        assert_eq!(content.load_utf8("text_file.txt").ok().unwrap(), "test text file");
        assert!(content.load_utf8("non_existing.txt").is_err());
    }

    #[test]
    fn read_binary() {
        let mut content = Content::default();

        content.add_root(Path::new("resources").to_owned());

        let mut read = content.load_binary("text_file.txt").ok().unwrap();
        let mut contents = [0u8; 14];
        read.read_exact(&mut contents);

        assert_eq!(&contents, b"test text file");
        assert!(content.load_binary("non_existing.txt").is_err());
    }
}