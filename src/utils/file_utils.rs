// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use std::path::{Path, PathBuf};
use std::fs;
use crate::parser::ParseError;
use crate::data::ProcessStack;

/// Utility functions for file operations
/// 
/// Check if a file has ITF extension
pub fn is_itf_file<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase() == "itf")
        .unwrap_or(false)
}

/// Get ITF files in a directory
pub fn find_itf_files<P: AsRef<Path>>(dir_path: P) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut itf_files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_itf_file(&path) {
                itf_files.push(path);
            }
        }
    }
    
    itf_files.sort();
    Ok(itf_files)
}

/// Load and parse ITF file with error handling
pub fn load_itf_file<P: AsRef<Path>>(file_path: P) -> Result<ProcessStack, FileError> {
    let path = file_path.as_ref();
    
    // Check if file exists
    if !path.exists() {
        return Err(FileError::FileNotFound(path.to_path_buf()));
    }
    
    // Check if it's a file (not directory)
    if !path.is_file() {
        return Err(FileError::NotAFile(path.to_path_buf()));
    }
    
    // Check extension
    if !is_itf_file(path) {
        return Err(FileError::InvalidExtension(path.to_path_buf()));
    }
    
    // Read file content
    let content = fs::read_to_string(path)
        .map_err(|e| FileError::ReadError(path.to_path_buf(), e))?;
    
    // Parse content
    let stack = crate::parser::parse_itf_file(&content)
        .map_err(|e| FileError::ParseError(path.to_path_buf(), e))?;
    
    Ok(stack)
}

/// Get file size in bytes
pub fn get_file_size<P: AsRef<Path>>(file_path: P) -> Result<u64, std::io::Error> {
    let metadata = fs::metadata(file_path)?;
    Ok(metadata.len())
}

/// Get human-readable file size string
pub fn format_file_size(size_bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    const THRESHOLD: u64 = 1024;
    
    if size_bytes < THRESHOLD {
        return format!("{size_bytes} B");
    }
    
    let mut size = size_bytes as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Extract technology name from file path
pub fn extract_technology_name<P: AsRef<Path>>(file_path: P) -> Option<String> {
    file_path
        .as_ref()
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|s| s.to_string())
}

/// Create backup of file before modification
pub fn create_backup<P: AsRef<Path>>(file_path: P) -> Result<PathBuf, std::io::Error> {
    let path = file_path.as_ref();
    let backup_path = path.with_extension(
        format!("{}.backup", 
               path.extension()
                   .and_then(|ext| ext.to_str())
                   .unwrap_or("itf")
        )
    );
    
    fs::copy(path, &backup_path)?;
    Ok(backup_path)
}

/// Validate file before processing
pub fn validate_file<P: AsRef<Path>>(file_path: P) -> Result<FileInfo, FileError> {
    let path = file_path.as_ref();
    
    if !path.exists() {
        return Err(FileError::FileNotFound(path.to_path_buf()));
    }
    
    if !path.is_file() {
        return Err(FileError::NotAFile(path.to_path_buf()));
    }
    
    let size = get_file_size(path)
        .map_err(|e| FileError::ReadError(path.to_path_buf(), e))?;
    
    let is_itf = is_itf_file(path);
    let technology_name = extract_technology_name(path);
    
    Ok(FileInfo {
        path: path.to_path_buf(),
        size_bytes: size,
        size_formatted: format_file_size(size),
        is_itf_file: is_itf,
        technology_name,
    })
}

/// File information structure
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub size_formatted: String,
    pub is_itf_file: bool,
    pub technology_name: Option<String>,
}

/// File operation errors
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    
    #[error("Path is not a file: {0}")]
    NotAFile(PathBuf),
    
    #[error("Invalid file extension: {0}")]
    InvalidExtension(PathBuf),
    
    #[error("Failed to read file {0}: {1}")]
    ReadError(PathBuf, std::io::Error),
    
    #[error("Failed to parse file {0}: {1}")]
    ParseError(PathBuf, ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_itf_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(format!("{name}.itf"));
        let mut file = File::create(&file_path).unwrap();
        write!(file, "{content}").unwrap();
        file_path
    }

    #[test]
    fn test_is_itf_file() {
        assert!(is_itf_file("test.itf"));
        assert!(is_itf_file("TEST.ITF"));
        assert!(is_itf_file("/path/to/file.itf"));
        
        assert!(!is_itf_file("test.txt"));
        assert!(!is_itf_file("test"));
        assert!(!is_itf_file("test."));
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_extract_technology_name() {
        assert_eq!(
            extract_technology_name("test_tech.itf"),
            Some("test_tech".to_string())
        );
        assert_eq!(
            extract_technology_name("/path/to/advanced_28nm.itf"),
            Some("advanced_28nm".to_string())
        );
        assert_eq!(
            extract_technology_name("file"),
            Some("file".to_string())
        );
    }

    #[test]
    fn test_find_itf_files() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        create_test_itf_file(&temp_dir, "test1", "TECHNOLOGY = test1");
        create_test_itf_file(&temp_dir, "test2", "TECHNOLOGY = test2");
        
        // Create non-ITF file
        let txt_path = temp_dir.path().join("test.txt");
        File::create(txt_path).unwrap();
        
        let itf_files = find_itf_files(temp_dir.path()).unwrap();
        assert_eq!(itf_files.len(), 2);
        
        // Files should be sorted
        assert!(itf_files[0].file_name().unwrap().to_str().unwrap() <= 
                itf_files[1].file_name().unwrap().to_str().unwrap());
    }

    #[test]
    fn test_validate_file() {
        let temp_dir = TempDir::new().unwrap();
        let itf_path = create_test_itf_file(&temp_dir, "test", "TECHNOLOGY = test");
        
        let file_info = validate_file(&itf_path).unwrap();
        assert_eq!(file_info.path, itf_path);
        assert!(file_info.size_bytes > 0);
        assert!(file_info.is_itf_file);
        assert_eq!(file_info.technology_name, Some("test".to_string()));
        
        // Test non-existent file
        let non_existent = temp_dir.path().join("nonexistent.itf");
        let result = validate_file(non_existent);
        assert!(matches!(result, Err(FileError::FileNotFound(_))));
        
        // Test directory
        let result = validate_file(temp_dir.path());
        assert!(matches!(result, Err(FileError::NotAFile(_))));
    }

    #[test]
    fn test_get_file_size() {
        let temp_dir = TempDir::new().unwrap();
        let content = "TECHNOLOGY = test\nDIELECTRIC oxide {THICKNESS=1.0 ER=4.2}";
        let itf_path = create_test_itf_file(&temp_dir, "test", content);
        
        let size = get_file_size(&itf_path).unwrap();
        assert_eq!(size, content.len() as u64);
    }

    #[test]
    fn test_create_backup() {
        let temp_dir = TempDir::new().unwrap();
        let original_path = create_test_itf_file(&temp_dir, "original", "test content");
        
        let backup_path = create_backup(&original_path).unwrap();
        assert!(backup_path.exists());
        assert!(backup_path.file_name().unwrap().to_str().unwrap().contains("backup"));
        
        // Verify backup has same content
        let original_content = fs::read_to_string(&original_path).unwrap();
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(original_content, backup_content);
    }

    #[test]
    fn test_load_itf_file_errors() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test non-existent file
        let non_existent = temp_dir.path().join("nonexistent.itf");
        let result = load_itf_file(non_existent);
        assert!(matches!(result, Err(FileError::FileNotFound(_))));
        
        // Test invalid extension
        let txt_path = temp_dir.path().join("test.txt");
        File::create(&txt_path).unwrap();
        let result = load_itf_file(txt_path);
        assert!(matches!(result, Err(FileError::InvalidExtension(_))));
        
        // Test directory
        let result = load_itf_file(temp_dir.path());
        assert!(matches!(result, Err(FileError::NotAFile(_))));
        
        // Test invalid ITF content
        let bad_itf = create_test_itf_file(&temp_dir, "bad", "invalid content");
        let result = load_itf_file(bad_itf);
        assert!(matches!(result, Err(FileError::ParseError(_, _))));
    }

    #[test]
    fn test_load_valid_itf_file() {
        let temp_dir = TempDir::new().unwrap();
        let content = r#"
            TECHNOLOGY = test_tech
            DIELECTRIC oxide {THICKNESS=1.0 ER=4.2}
            CONDUCTOR metal {THICKNESS=0.5 RPSQ=0.1}
        "#;
        let itf_path = create_test_itf_file(&temp_dir, "valid", content);
        
        let stack = load_itf_file(itf_path).unwrap();
        assert_eq!(stack.technology_info.name, "test_tech");
        assert_eq!(stack.get_layer_count(), 2);
    }
}