//! This file defines a `FileIter` struct which is a circular iterator over all files in a given directory

use std::iter::Cycle;
use std::path::PathBuf;
use std::vec::IntoIter;

pub struct FileIter {
    iter: Cycle<IntoIter<PathBuf>>,
}

impl FileIter {
    pub fn new(dir: &str) -> std::io::Result<Self> {
        let files: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.is_file())
            .collect();
        Ok(Self {
            iter: files.into_iter().cycle(),
        })
    }

    pub fn with_extension(dir: &str, ext: &str) -> std::io::Result<Self> {
        let files: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.is_file() && path.extension().and_then(|e| e.to_str()) == Some(ext))
            .collect();
        Ok(Self {
            iter: files.into_iter().cycle(),
        })
    }
}

impl Iterator for FileIter {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
