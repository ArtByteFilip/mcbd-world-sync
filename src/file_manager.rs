use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub size: u64,
    pub hash: String,
}

pub struct FileManager {
    base_path: PathBuf,
    file_cache: HashMap<PathBuf, FileInfo>,
}

impl FileManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            file_cache: HashMap::new(),
        }
    }

    pub fn scan_directory(&mut self) -> Result<Vec<FileInfo>> {
        let mut files = Vec::new();
        let base_path = self.base_path.clone();
        self.scan_directory_recursive(&base_path, &mut files)?;
        Ok(files)
    }

    fn scan_directory_recursive(&mut self, dir: &Path, files: &mut Vec<FileInfo>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.scan_directory_recursive(&path, files)?;
            } else {
                if let Ok(metadata) = fs::metadata(&path) {
                    let relative_path = path.strip_prefix(&self.base_path)?;
                    let file_info = FileInfo {
                        path: relative_path.to_path_buf(),
                        last_modified: metadata.modified()?,
                        size: metadata.len(),
                        hash: self.calculate_file_hash(&path)?,
                    };
                    files.push(file_info.clone());
                    self.file_cache.insert(relative_path.to_path_buf(), file_info);
                }
            }
        }
        Ok(())
    }

    pub fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};
        let mut file = fs::File::open(path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    pub fn get_file_content(&self, path: &Path) -> Result<Vec<u8>> {
        let full_path = self.base_path.join(path);
        Ok(fs::read(full_path)?)
    }

    pub fn save_file_content(&self, path: &Path, content: &[u8]) -> Result<()> {
        let full_path = self.base_path.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, content)?;
        Ok(())
    }

    pub fn get_file_info(&self, path: &Path) -> Option<&FileInfo> {
        self.file_cache.get(path)
    }

    pub fn update_file_info(&mut self, path: PathBuf, info: FileInfo) {
        self.file_cache.insert(path, info);
    }

    pub fn handle_conflict(&self, local: &FileInfo, remote: &FileInfo) -> Result<FileInfo> {
        // Simple conflict resolution: use the newest file
        if local.last_modified > remote.last_modified {
            Ok(local.clone())
        } else {
            Ok(remote.clone())
        }
    }
} 