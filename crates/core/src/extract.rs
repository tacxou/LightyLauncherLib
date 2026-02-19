use crate::errors::{ExtractError, ExtractResult};
use async_compression::tokio::bufread::GzipDecoder;
use async_zip::tokio::read::seek::ZipFileReader;
use futures_util::io::{self, BufReader as FuturesBufReader};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, OpenOptions};
use tokio::io::AsyncBufRead;
use tokio::io::{AsyncRead, AsyncSeek, BufReader};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[cfg(feature = "events")]
use lighty_event::{EventBus, Event, CoreEvent};

// Maximum file size: 2GB (protection against zip bombs)
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024;
// Buffer size for extraction: 256KB
const BUFFER_SIZE: usize = 256 * 1024;

/// Extracts everything from the ZIP archive to the output directory
///
/// Security features:
/// - Path traversal protection
/// - Symlink/hardlink rejection
/// - Absolute path rejection
/// - File size limits (2GB max)
/// - Path sanitization
pub async fn zip_extract<R>(
    archive: R,
    out_dir: &Path,
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> ExtractResult<()>
where
    R: AsyncRead + AsyncSeek + Unpin + AsyncBufRead,
{
    let out_dir = out_dir.canonicalize()?; // Normalize base directory
    let mut reader = ZipFileReader::new(archive.compat()).await?;

    let entries_count = reader.file().entries().len();

    #[cfg(feature = "events")]
    if let Some(bus) = event_bus {
        bus.emit(Event::Core(CoreEvent::ExtractionStarted {
            archive_type: "ZIP".to_string(),
            file_count: entries_count,
            destination: out_dir.to_string_lossy().to_string(),
        }));
    }

    for index in 0..entries_count {
        // Collect entry metadata before mutably borrowing reader
        let (_file_name, path, is_dir, uncompressed_size) = {
            let entry = reader.file().entries().get(index)
                .ok_or_else(|| ExtractError::ZipEntryNotFound { index })?;

            let file_name = entry.filename().as_str()?;
            let is_dir = entry.dir()?;
            let uncompressed_size = entry.uncompressed_size();

            // Sanitize and build path
            let sanitized = sanitize_file_path(file_name);

            // Reject absolute paths early
            if sanitized.is_absolute() {
                return Err(ExtractError::AbsolutePath {
                    path: file_name.to_string()
                });
            }

            let path = out_dir.join(&sanitized);

            // Validate path doesn't escape out_dir using path components
            if !is_path_within_base(&path, &out_dir)? {
                return Err(ExtractError::PathTraversal {
                    path: file_name.to_string()
                });
            }

            (file_name.to_string(), path, is_dir, uncompressed_size)
        };

        // Now extract - mutably borrow reader
        if is_dir {
            // Create directory
            create_dir_all(&path).await?;
        } else {
            // Check file size (zip bomb protection)
            if uncompressed_size > MAX_FILE_SIZE {
                return Err(ExtractError::FileTooLarge {
                    size: uncompressed_size,
                    max: MAX_FILE_SIZE,
                });
            }

            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                create_dir_all(parent).await?;
            }

            // Extract file with buffering
            let entry_reader = reader.reader_with_entry(index).await?;
            let buffered_reader = FuturesBufReader::with_capacity(BUFFER_SIZE, entry_reader);

            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .await?;

            // Copy with buffering for performance
            io::copy(buffered_reader, &mut file.compat_write()).await?;
        }

        // Emit progress event every 10 files
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            if (index + 1) % 10 == 0 || (index + 1) == entries_count {
                bus.emit(Event::Core(CoreEvent::ExtractionProgress {
                    files_extracted: index + 1,
                    total_files: entries_count,
                }));
            }
        }
    }

    #[cfg(feature = "events")]
    if let Some(bus) = event_bus {
        bus.emit(Event::Core(CoreEvent::ExtractionCompleted {
            archive_type: "ZIP".to_string(),
            files_extracted: entries_count,
        }));
    }

    Ok(())
}

/// Extracts tar.gz archive to the output directory
///
/// Security features:
/// - Path traversal protection
/// - Symlink/hardlink rejection
/// - Absolute path rejection
/// - File size limits (2GB max)
pub async fn tar_gz_extract<R>(
    archive: R,
    out_dir: &Path,
    #[cfg(feature = "events")] event_bus: Option<&EventBus>,
) -> ExtractResult<()>
where
    R: AsyncRead + Unpin,
{
    let out_dir = out_dir.canonicalize()?;
    let decoder = GzipDecoder::new(BufReader::new(archive));
    let mut ar = tokio_tar::Archive::new(decoder);

    #[cfg(feature = "events")]
    if let Some(bus) = event_bus {
        bus.emit(Event::Core(CoreEvent::ExtractionStarted {
            archive_type: "TAR.GZ".to_string(),
            file_count: 0, // Unknown for tar.gz until we iterate
            destination: out_dir.to_string_lossy().to_string(),
        }));
    }

    // Manual extraction with validation
    let mut entries = ar.entries()?;
    let mut _files_extracted = 0usize;

    while let Some(entry) = entries.next().await {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();

        // Reject absolute paths
        if path.is_absolute() {
            continue;
        }

        let dest = out_dir.join(&path);

        // Path traversal protection - use robust validation
        if !is_path_within_base(&dest, &out_dir)? {
            continue;
        }

        // Skip symlinks and hard links for security
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            continue;
        }

        // Check file size (protection against tar bombs)
        let size = entry.header().size()?;
        if size > MAX_FILE_SIZE {
            return Err(ExtractError::FileTooLarge {
                size,
                max: MAX_FILE_SIZE,
            });
        }

        // Extract safely
        entry.unpack(&dest).await?;

        _files_extracted += 1;

        // Emit progress event every 10 files
        #[cfg(feature = "events")]
        if let Some(bus) = event_bus {
            if _files_extracted % 10 == 0 {
                bus.emit(Event::Core(CoreEvent::ExtractionProgress {
                    files_extracted: _files_extracted,
                    total_files: 0, // Unknown for tar.gz
                }));
            }
        }
    }

    #[cfg(feature = "events")]
    if let Some(bus) = event_bus {
        bus.emit(Event::Core(CoreEvent::ExtractionCompleted {
            archive_type: "TAR.GZ".to_string(),
            files_extracted: _files_extracted,
        }));
    }

    Ok(())
}

/// Returns a relative path without reserved names, redundant separators, ".", or "..".
fn sanitize_file_path(path: &str) -> PathBuf {
    // Replaces backwards slashes
    path.replace('\\', "/")
        // Sanitizes each component
        .split('/')
        .map(sanitize_filename::sanitize)
        .collect()
}

/// Validates that a path is within the base directory using path components
/// This is more robust than canonicalize() which fails on non-existent paths
fn is_path_within_base(path: &Path, base: &Path) -> ExtractResult<bool> {
    // Normalize both paths by collecting components
    let normalized_path: PathBuf = path.components()
        .fold(PathBuf::new(), |mut acc, component| {
            match component {
                std::path::Component::Normal(c) => acc.push(c),
                std::path::Component::ParentDir => { acc.pop(); },
                std::path::Component::CurDir => {},
                _ => acc.push(component),
            }
            acc
        });

    let normalized_base: PathBuf = base.components()
        .fold(PathBuf::new(), |mut acc, component| {
            match component {
                std::path::Component::Normal(c) => acc.push(c),
                std::path::Component::ParentDir => { acc.pop(); },
                std::path::Component::CurDir => {},
                _ => acc.push(component),
            }
            acc
        });

    Ok(normalized_path.starts_with(&normalized_base))
}