use std::{path::{Path, PathBuf}, collections::HashSet};
use tokio::fs;
use crate::{error::McpError, types::FileInfo};
use futures::stream::{self, StreamExt};
use path_clean::clean;

pub struct FileSystemManager {
    allowed_directories: HashSet<PathBuf>,
}

impl FileSystemManager {
    pub fn new(allowed_dirs: Vec<PathBuf>) -> Result<Self, McpError> {
        let mut normalized_dirs = HashSet::new();
        
        for dir in allowed_dirs {
            let normalized = clean(&dir).to_string_lossy().to_lowercase();
            normalized_dirs.insert(PathBuf::from(normalized));
            
            // Validate directory exists and is accessible
            if !dir.is_dir() {
                return Err(McpError::InvalidRequest(format!("{:?} is not a directory", dir)));
            }
        }
        
        Ok(Self {
            allowed_directories: normalized_dirs,
        })
    }

    pub async fn validate_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, McpError> {
        let path = path.as_ref();
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        
        let normalized = clean(&absolute).to_string_lossy().to_lowercase();
        
        // Check if path is within allowed directories
        if !self.allowed_directories.iter().any(|dir| normalized.starts_with(dir.to_string_lossy().as_ref())) {
            return Err(McpError::AccessDenied(format!(
                "Path outside allowed directories: {:?}",
                absolute
            )));
        }

        Ok(absolute)
    }

    pub async fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<String, McpError> {
        let valid_path = self.validate_path(path).await?;
        Ok(fs::read_to_string(valid_path).await?)
    }

    pub async fn read_multiple_files(&self, paths: Vec<String>) -> Result<Vec<(String, Result<String, String>)>, McpError> {
        let results = stream::iter(paths)
            .map(|path| async move {
                let result = match self.read_file(&path).await {
                    Ok(content) => Ok(content),
                    Err(e) => Err(e.to_string()),
                };
                (path, result)
            })
            .buffer_unordered(10)
            .collect::<Vec<_>>()
            .await;
            
        Ok(results)
    }

    pub async fn write_file<P: AsRef<Path>>(&self, path: P, content: String) -> Result<(), McpError> {
        let valid_path = self.validate_path(path).await?;
        Ok(fs::write(valid_path, content).await?)
    }

    pub async fn create_directory<P: AsRef<Path>>(&self, path: P) -> Result<(), McpError> {
        let valid_path = self.validate_path(path).await?;
        Ok(fs::create_dir_all(valid_path).await?)
    }

    pub async fn list_directory<P: AsRef<Path>>(&self, path: P) -> Result<Vec<String>, McpError> {
        let valid_path = self.validate_path(path).await?;
        let mut entries = Vec::new();
        
        let mut read_dir = fs::read_dir(valid_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let file_type = entry.file_type().await?;
            let prefix = if file_type.is_dir() { "[DIR]" } else { "[FILE]" };
            entries.push(format!("{} {}", prefix, entry.file_name().to_string_lossy()));
        }
        
        Ok(entries)
    }

    pub async fn move_file<P: AsRef<Path>>(&self, source: P, destination: P) -> Result<(), McpError> {
        let valid_source = self.validate_path(source).await?;
        let valid_dest = self.validate_path(destination).await?;
        Ok(fs::rename(valid_source, valid_dest).await?)
    }

    pub async fn search_files<P: AsRef<Path>>(&self, root: P, pattern: &str) -> Result<Vec<String>, McpError> {
        let valid_root = self.validate_path(root).await?;
        let pattern = pattern.to_lowercase();
        let mut results = Vec::new();
        
        async fn search_dir(manager: &FileSystemManager, dir: PathBuf, pattern: &str, results: &mut Vec<String>) -> Result<(), McpError> {
            let mut read_dir = fs::read_dir(&dir).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let path = entry.path();
                
                if let Ok(_) = manager.validate_path(&path).await {
                    if path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.to_lowercase().contains(&pattern))
                        .unwrap_or(false)
                    {
                        results.push(path.to_string_lossy().to_string());
                    }
                    
                    if path.is_dir() {
                        search_dir(manager, path, pattern, results).await?;
                    }
                }
            }
            Ok(())
        }
        
        search_dir(self, valid_root, &pattern, &mut results).await?;
        Ok(results)
    }

    pub async fn get_file_info<P: AsRef<Path>>(&self, path: P) -> Result<FileInfo, McpError> {
        let valid_path = self.validate_path(path).await?;
        let metadata = fs::metadata(valid_path).await?;
        
        Ok(FileInfo {
            size: metadata.len(),
            created: metadata.created()?,
            modified: metadata.modified()?,
            accessed: metadata.accessed()?,
            is_directory: metadata.is_dir(),
            is_file: metadata.is_file(),
            permissions: format!("{:o}", metadata.permissions().mode() & 0o777),
        })
    }

    pub fn list_allowed_directories(&self) -> Vec<String> {
        self.allowed_directories
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }
}
