//! File upload handling for RustF
//!
//! This module provides Total.js-style file upload handling with support for
//! multipart/form-data parsing and file validation.

use crate::error::{Error, Result};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// Represents an uploaded file
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// Original filename as provided by the client
    pub filename: Option<String>,

    /// Content type (MIME type) of the file
    pub content_type: Option<String>,

    /// Size of the file in bytes
    pub size: usize,

    /// File contents as bytes
    pub data: Vec<u8>,

    /// Form field name
    pub field_name: String,
}

impl UploadedFile {
    /// Create a new uploaded file
    pub fn new(
        field_name: String,
        filename: Option<String>,
        content_type: Option<String>,
        data: Vec<u8>,
    ) -> Self {
        let size = data.len();
        Self {
            filename,
            content_type,
            size,
            data,
            field_name,
        }
    }

    /// Get the file extension from filename
    pub fn extension(&self) -> Option<&str> {
        self.filename
            .as_ref()
            .and_then(|name| Path::new(name).extension())
            .and_then(|ext| ext.to_str())
    }

    /// Check if file is an image based on content type
    pub fn is_image(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.starts_with("image/"))
            .unwrap_or(false)
    }

    /// Check if file is a document based on content type
    pub fn is_document(&self) -> bool {
        if let Some(ct) = &self.content_type {
            ct.starts_with("application/pdf")
                || ct.starts_with("application/msword")
                || ct.starts_with("application/vnd.openxmlformats-officedocument")
                || ct.starts_with("text/")
        } else {
            false
        }
    }

    /// Save file to disk
    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let base_dir = Self::default_upload_base_dir()?;
        self.save_to_from(base_dir, path)
    }

    /// Save a file relative to a constrained base directory.
    pub fn save_to_from<B: AsRef<Path>, P: AsRef<Path>>(&self, base_dir: B, path: P) -> Result<()> {
        let safe_path = Self::resolve_contained_output_path(base_dir.as_ref(), path.as_ref())?;
        self.write_to_path(&safe_path)
    }

    /// Save file to an arbitrary filesystem path.
    pub fn save_to_external<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.write_to_path(path.as_ref())
    }

    /// Return a storage-safe filename derived from the client-provided name.
    pub fn safe_filename(&self) -> Option<String> {
        self.filename
            .as_deref()
            .map(Self::sanitize_filename)
            .filter(|name| !name.is_empty())
    }

    fn write_to_path(&self, path: &Path) -> Result<()> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(&self.data)?;
        Ok(())
    }

    /// Get file contents as string (for text files)
    pub fn as_string(&self) -> Result<String> {
        String::from_utf8(self.data.clone())
            .map_err(|_| Error::InvalidInput("File is not valid UTF-8".to_string()))
    }

    /// Validate file size
    pub fn validate_size(&self, max_size: usize) -> Result<()> {
        if self.size > max_size {
            return Err(Error::InvalidInput(format!(
                "File size {} exceeds maximum allowed size {}",
                self.size, max_size
            )));
        }
        Ok(())
    }

    /// Validate file type by extension
    pub fn validate_extension(&self, allowed_extensions: &[&str]) -> Result<()> {
        if let Some(ext) = self.extension() {
            let ext_lower = ext.to_lowercase();
            if allowed_extensions
                .iter()
                .any(|&allowed| allowed.to_lowercase() == ext_lower)
            {
                Ok(())
            } else {
                Err(Error::InvalidInput(format!(
                    "File extension '{}' is not allowed. Allowed: {:?}",
                    ext, allowed_extensions
                )))
            }
        } else {
            Err(Error::InvalidInput("File has no extension".to_string()))
        }
    }

    /// Validate content type
    pub fn validate_content_type(&self, allowed_types: &[&str]) -> Result<()> {
        if let Some(ct) = &self.content_type {
            if allowed_types.iter().any(|&allowed| ct.starts_with(allowed)) {
                Ok(())
            } else {
                Err(Error::InvalidInput(format!(
                    "Content type '{}' is not allowed. Allowed: {:?}",
                    ct, allowed_types
                )))
            }
        } else {
            Err(Error::InvalidInput("File has no content type".to_string()))
        }
    }

    fn default_upload_base_dir() -> Result<PathBuf> {
        if let Some(upload_dir) = crate::configuration::CONF::get_string("uploads.directory") {
            return Ok(PathBuf::from(upload_dir));
        }

        Ok(std::env::current_dir()
            .map_err(Error::Io)?
            .join("private")
            .join("uploads"))
    }

    fn resolve_contained_output_path(base_dir: &Path, path: &Path) -> Result<PathBuf> {
        let canonical_base = base_dir.canonicalize().map_err(Error::Io)?;
        let relative_path = Self::normalize_relative_path(path)?;
        let target_path = canonical_base.join(&relative_path);

        let parent = target_path.parent().ok_or_else(|| {
            Error::InvalidInput("Upload path must include a target filename".to_string())
        })?;

        if crate::configuration::CONF::get_bool("uploads.create_directories").unwrap_or(true) {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }

        let canonical_parent = parent.canonicalize().map_err(Error::Io)?;
        if !canonical_parent.starts_with(&canonical_base) {
            return Err(Error::InvalidInput(
                "Upload path escapes the configured uploads directory".to_string(),
            ));
        }

        Ok(target_path)
    }

    fn normalize_relative_path(path: &Path) -> Result<PathBuf> {
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(segment) => {
                    let segment_str = segment.to_string_lossy();
                    if segment_str.is_empty()
                        || segment_str.contains('/')
                        || segment_str.contains('\\')
                        || segment_str.chars().any(|ch| ch.is_control())
                    {
                        return Err(Error::InvalidInput(
                            "Upload path contains invalid path segments".to_string(),
                        ));
                    }
                    normalized.push(segment);
                }
                _ => {
                    return Err(Error::InvalidInput(
                        "Upload path must stay within the configured uploads directory".to_string(),
                    ))
                }
            }
        }

        if normalized.as_os_str().is_empty() {
            return Err(Error::InvalidInput(
                "Upload path must include a target filename".to_string(),
            ));
        }

        Ok(normalized)
    }

    fn sanitize_filename(filename: &str) -> String {
        let basename = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
        let sanitized: String = basename
            .chars()
            .filter(|&ch| !matches!(ch, '\r' | '\n' | '\0') && !ch.is_control())
            .map(|ch| match ch {
                '/' | '\\' | ':' => '_',
                _ => ch,
            })
            .collect();

        sanitized.trim_matches('.').trim().to_string()
    }
}

/// Collection of uploaded files
#[derive(Debug, Default)]
pub struct FileCollection {
    files: HashMap<String, Vec<UploadedFile>>,
}

impl FileCollection {
    /// Create new empty file collection
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Add a file to the collection
    pub fn add(&mut self, file: UploadedFile) {
        self.files
            .entry(file.field_name.clone())
            .or_default()
            .push(file);
    }

    /// Get first file by field name
    pub fn get(&self, field_name: &str) -> Option<&UploadedFile> {
        self.files.get(field_name)?.first()
    }

    /// Get all files by field name
    pub fn get_all(&self, field_name: &str) -> Option<&Vec<UploadedFile>> {
        self.files.get(field_name)
    }

    /// Get all files as a flat iterator
    pub fn all(&self) -> impl Iterator<Item = &UploadedFile> {
        self.files.values().flatten()
    }

    /// Get field names
    pub fn field_names(&self) -> impl Iterator<Item = &String> {
        self.files.keys()
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Get total number of files
    pub fn len(&self) -> usize {
        self.files.values().map(|v| v.len()).sum()
    }

    /// Get total size of all files
    pub fn total_size(&self) -> usize {
        self.all().map(|f| f.size).sum()
    }
}

/// Simple multipart parser for file uploads
pub struct MultipartParser;

impl MultipartParser {
    /// Parse multipart form data from request body
    pub fn parse(body: &[u8], boundary: &str) -> Result<(FileCollection, HashMap<String, String>)> {
        let boundary_bytes = format!("--{}", boundary).into_bytes();
        let mut files = FileCollection::new();
        let mut form_data = HashMap::new();

        // Simple multipart parsing (basic implementation)
        let parts = Self::split_multipart(body, &boundary_bytes);

        for part in parts {
            if let Some((headers, body)) = Self::parse_part(&part) {
                if let Some(content_disposition) = headers.get("content-disposition") {
                    if let Some(field_name) = Self::extract_field_name(content_disposition) {
                        if let Some(filename) = Self::extract_filename(content_disposition) {
                            // This is a file upload
                            let content_type = headers.get("content-type").cloned();
                            let file =
                                UploadedFile::new(field_name, Some(filename), content_type, body);
                            files.add(file);
                        } else {
                            // This is regular form data
                            if let Ok(value) = String::from_utf8(body) {
                                form_data.insert(field_name, value);
                            }
                        }
                    }
                }
            }
        }

        Ok((files, form_data))
    }

    fn split_multipart(body: &[u8], boundary: &[u8]) -> Vec<Vec<u8>> {
        let mut parts = Vec::new();
        let mut start = 0;

        while let Some(pos) = Self::find_bytes(body, boundary, start) {
            if start > 0 {
                let part = body[start..pos].to_vec();
                if !part.is_empty() {
                    parts.push(part);
                }
            }
            start = pos + boundary.len();
        }

        parts
    }

    fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
        if start >= haystack.len() {
            return None;
        }

        (start..=haystack.len().saturating_sub(needle.len()))
            .find(|&i| haystack[i..i + needle.len()] == *needle)
    }

    fn parse_part(part: &[u8]) -> Option<(HashMap<String, String>, Vec<u8>)> {
        // Find double CRLF that separates headers from body
        let separator = b"\r\n\r\n";
        if let Some(pos) = Self::find_bytes(part, separator, 0) {
            let headers_bytes = &part[..pos];
            let mut body = part[pos + separator.len()..].to_vec();
            if body.ends_with(b"\r\n") {
                body.truncate(body.len() - 2);
            }

            let mut headers = HashMap::new();
            let headers_str = String::from_utf8_lossy(headers_bytes);
            for line in headers_str.lines() {
                if let Some((key, value)) = line.split_once(": ") {
                    headers.insert(key.to_lowercase(), value.to_string());
                }
            }

            Some((headers, body))
        } else {
            None
        }
    }

    fn extract_field_name(content_disposition: &str) -> Option<String> {
        // Parse: form-data; name="field_name"; filename="file.txt"
        for part in content_disposition.split(';') {
            let part = part.trim();
            if part.starts_with("name=") {
                return Some(part[5..].trim_matches('"').to_string());
            }
        }
        None
    }

    fn extract_filename(content_disposition: &str) -> Option<String> {
        for part in content_disposition.split(';') {
            let part = part.trim();
            if part.starts_with("filename=") {
                return Some(part[9..].trim_matches('"').to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::UploadedFile;
    use crate::error::Error;

    #[test]
    fn safe_filename_strips_path_components_and_controls() {
        let file = UploadedFile::new(
            "avatar".to_string(),
            Some("..\\..\\evil\r\n.png".to_string()),
            Some("image/png".to_string()),
            vec![1, 2, 3],
        );

        assert_eq!(file.safe_filename().as_deref(), Some("evil.png"));
    }

    #[test]
    fn save_to_from_rejects_parent_traversal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = UploadedFile::new("doc".to_string(), None, None, b"hello".to_vec());

        let err = file
            .save_to_from(temp_dir.path(), "../secret.txt")
            .unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn save_to_from_writes_under_base_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = UploadedFile::new("doc".to_string(), None, None, b"hello".to_vec());

        file.save_to_from(temp_dir.path(), "nested/file.txt")
            .unwrap();

        assert_eq!(
            std::fs::read(temp_dir.path().join("nested/file.txt")).unwrap(),
            b"hello"
        );
    }
}
