//! Recursively copy a directory from a to b.
//! ```no_run
//! use dircpy::*;
//!
//! // Most basic example:
//! copy_dir("src", "dest").unwrap();
//!
//! // Simple builder example:
//!CopyBuilder::new("src", "dest")
//!.run()
//!.unwrap();
//!
//! // Copy recursively, only including certain files:
//!CopyBuilder::new("src", "dest")
//!.overwrite_if_newer(true)
//!.overwrite_if_size_differs(true)
//!.with_include_filter(".txt")
//!.with_include_filter(".csv")
//!.run()
//!.unwrap();
//! ```

use log::*;
// use rayon::prelude::*;
use std::fs::{copy, read_link};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use walkdir::WalkDir;

#[cfg(test)]
mod tests;

/// A function to output progress in the form (total entries, processed entries).
/// Entries include directories and files that are skipped.
type ProgressFn = Arc<dyn Fn(usize, usize)>;

#[derive(Clone)]
/// Recursively copy a directory from a to b.
/// ```no_run
/// use dircpy::*;
///
/// // Most basic example:
/// copy_dir("src", "dest");
///
/// // Simple builder example:
///CopyBuilder::new("src", "dest")
///.run()
///.unwrap();
///
/// // Copy recursively, only including certain files:
///CopyBuilder::new("src", "dest")
///.overwrite_if_newer(true)
///.overwrite_if_size_differs(true)
///.with_include_filter(".txt")
///.with_include_filter(".csv")
///.run()
///.unwrap();
///
/// // Copy with progress:
///CopyBuilder::new("src", "dest")
///.with_progress(|all, done| {
///    println!("copied {done}/{all}");
///})
///.run()
///.unwrap();
///
/// ```
pub struct CopyBuilder {
    /// The source directory
    pub source: PathBuf,
    /// The destination directory
    pub destination: PathBuf,
    /// Overwrite all files in target, if already existing
    overwrite_all: bool,
    /// Overwrite target files if the source file is newer
    overwrite_if_newer: bool,
    /// Overwrite target files if they differ in size
    overwrite_if_size_differs: bool,
    /// A list of exclude filters
    exclude_filters: Vec<String>,
    /// A list of include filters
    include_filters: Vec<String>,
    /// An optional progress function. Has a performance penalty as the total number of entries needs to be calculated.
    progress_callback: Option<ProgressFn>,
}

impl std::fmt::Debug for CopyBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopyBuilder")
            .field("source", &self.source)
            .field("destination", &self.destination)
            .field("overwrite_all", &self.overwrite_all)
            .field("overwrite_if_newer", &self.overwrite_if_newer)
            .field("overwrite_if_size_differs", &self.overwrite_if_size_differs)
            .field("exclude_filters", &self.exclude_filters)
            .field("include_filters", &self.include_filters)
            .field("has_progress_callback", &self.progress_callback.is_some())
            .finish()
    }
}

/// Determine if the modification date of file_a is newer than that of file_b
fn is_file_newer(file_a: &Path, file_b: &Path) -> bool {
    match (file_a.symlink_metadata(), file_b.symlink_metadata()) {
        (Ok(meta_a), Ok(meta_b)) => {
            meta_a.modified().unwrap_or_else(|_| SystemTime::now())
                > meta_b.modified().unwrap_or(SystemTime::UNIX_EPOCH)
        }
        _ => false,
    }
}

/// Determine if file_a and file_b's size differs.
fn is_filesize_different(file_a: &Path, file_b: &Path) -> bool {
    match (file_a.symlink_metadata(), file_b.symlink_metadata()) {
        (Ok(meta_a), Ok(meta_b)) => meta_a.len() != meta_b.len(),
        _ => false,
    }
}

/// Determine if path is a symlink, without following it.
fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Remove a file or symlink without following it. On Windows, symlinks
/// to directories need to be removed with `remove_dir`.
fn remove_symlink(path: &Path) -> Result<(), std::io::Error> {
    std::fs::remove_file(path)
        .or_else(|e| {
            if cfg!(windows) && is_symlink(path) {
                std::fs::remove_dir(path)
            } else {
                Err(e)
            }
        })
        .map_err(|e| context(e, format!("Could not remove {}", path.display())))
}

/// Create a symlink at `link` pointing to `target`. `original` is the symlink being copied.
#[cfg(unix)]
fn create_symlink(_original: &Path, target: &Path, link: &Path) -> Result<(), std::io::Error> {
    std::os::unix::fs::symlink(target, link)
}

/// Create a symlink at `link` pointing to `target`. `original` is the symlink being copied.
/// Windows distinguishes between file and directory symlinks, so the type of `original` is used.
#[cfg(windows)]
fn create_symlink(original: &Path, target: &Path, link: &Path) -> Result<(), std::io::Error> {
    use std::os::windows::fs::FileTypeExt;
    if original.symlink_metadata()?.file_type().is_symlink_dir() {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// Create a symlink at `link` pointing to `target`. `original` is the symlink being copied.
#[cfg(not(any(unix, windows)))]
fn create_symlink(_original: &Path, _target: &Path, _link: &Path) -> Result<(), std::io::Error> {
    Err(Error::new(
        ErrorKind::Unsupported,
        "Symlinks are not supported on this platform",
    ))
}

/// Prefix an error with a description, usually containing the affected path. Keeps the error kind.
fn context(e: Error, description: String) -> Error {
    Error::new(e.kind(), format!("{description}: {e}"))
}

impl CopyBuilder {
    /// Construct a new CopyBuilder with `source` and `dest`.
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(source: P, dest: Q) -> CopyBuilder {
        CopyBuilder {
            source: source.as_ref().to_path_buf(),
            destination: dest.as_ref().to_path_buf(),
            overwrite_all: false,
            overwrite_if_newer: false,
            overwrite_if_size_differs: false,
            exclude_filters: vec![],
            include_filters: vec![],
            progress_callback: None,
        }
    }

    /// Overwrite target files (off by default). Takes precedence over the conditional overwrite options.
    pub fn overwrite(self, overwrite: bool) -> CopyBuilder {
        CopyBuilder {
            overwrite_all: overwrite,
            ..self
        }
    }

    /// Overwrite if the source is newer (off by default)
    pub fn overwrite_if_newer(self, overwrite_only_newer: bool) -> CopyBuilder {
        CopyBuilder {
            overwrite_if_newer: overwrite_only_newer,
            ..self
        }
    }

    /// Overwrite if size between source and dest differs (off by default)
    pub fn overwrite_if_size_differs(self, overwrite_if_size_differs: bool) -> CopyBuilder {
        CopyBuilder {
            overwrite_if_size_differs,
            ..self
        }
    }

    /// Supply a callback function to be executed for each entry in the source directory.
    /// It supplies the total number of entries and the number of entries already processed.
    /// Entries include directories and files that are skipped.
    pub fn with_progress<F>(self, callback: F) -> CopyBuilder
    where
        F: Fn(usize, usize) + 'static,
    {
        CopyBuilder {
            progress_callback: Some(Arc::new(callback)),
            ..self
        }
    }

    /// Do not copy files and directories whose path relative to source contains this string.
    pub fn with_exclude_filter(self, f: &str) -> CopyBuilder {
        let mut filters = self.exclude_filters.clone();
        filters.push(f.to_owned());
        CopyBuilder {
            exclude_filters: filters,
            ..self
        }
    }

    /// Only copy files whose path relative to source contains this string.
    /// Directories are only created if they match, or if a file in them is copied.
    pub fn with_include_filter(self, f: &str) -> CopyBuilder {
        let mut filters = self.include_filters.clone();
        filters.push(f.to_owned());
        CopyBuilder {
            include_filters: filters,
            ..self
        }
    }

    /// Determine if a path relative to source matches an exclude filter
    fn is_excluded(&self, rel_path: &Path) -> bool {
        let rel_path = rel_path.to_string_lossy();
        self.exclude_filters
            .iter()
            .any(|f| rel_path.contains(f.as_str()))
    }

    /// Determine if a path relative to source matches an include filter, or if there are none
    fn is_included(&self, rel_path: &Path) -> bool {
        let rel_path = rel_path.to_string_lossy();
        self.include_filters.is_empty()
            || self
                .include_filters
                .iter()
                .any(|f| rel_path.contains(f.as_str()))
    }

    /// Execute the copy operation
    pub fn run(&self) -> Result<(), std::io::Error> {
        // Resolve source first, so dest is not created if source is missing
        let abs_source = self.source.canonicalize().map_err(|e| {
            context(
                e,
                format!("Could not read source {}", self.source.display()),
            )
        })?;
        if !abs_source.is_dir() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Source {} is not a directory", self.source.display()),
            ));
        }
        if !self.destination.is_dir() {
            debug!("MKDIR {:?}", &self.destination);
            std::fs::create_dir_all(&self.destination).map_err(|e| {
                context(
                    e,
                    format!(
                        "Could not create destination {}",
                        self.destination.display()
                    ),
                )
            })?;
        }
        let abs_dest = self.destination.canonicalize().map_err(|e| {
            context(
                e,
                format!("Could not read destination {}", self.destination.display()),
            )
        })?;
        debug!(
            "Building copy operation: SRC {} DST {}",
            abs_source.display(),
            abs_dest.display()
        );

        // Skip dest if it is inside source, and excluded entries including their contents
        let walk = || {
            WalkDir::new(&abs_source).into_iter().filter_entry(|e| {
                e.path() != abs_dest
                    && !self.is_excluded(e.path().strip_prefix(&abs_source).unwrap_or(e.path()))
            })
        };

        let mut num_files_total = 1;
        let mut num_files_processed = 0;

        if self.progress_callback.is_some() {
            num_files_total = walk().filter_map(|e| e.ok()).count();
        }

        for entry in walk() {
            // Don't ignore errors, as this would silently result in an incomplete copy
            let entry = entry?;
            if let Some(cb) = &self.progress_callback {
                num_files_processed += 1;
                cb(num_files_total, num_files_processed);
            }

            let rel_dest = entry.path().strip_prefix(&abs_source).map_err(|e| {
                Error::new(ErrorKind::Other, format!("Could not strip prefix: {:?}", e))
            })?;
            let dest_entry = abs_dest.join(rel_dest);

            if entry.path().symlink_metadata().is_ok() && !entry.file_type().is_dir() {
                // the source exists, but isn't a directory

                if !entry.file_type().is_file() && !entry.file_type().is_symlink() {
                    // Sockets, fifos and devices can't be copied meaningfully
                    warn!(
                        "Skipping {}: unsupported file type {:?}",
                        entry.path().display(),
                        entry.file_type()
                    );
                    continue;
                }

                // Early out if target is present and overwrite is off
                if !self.overwrite_all
                    && dest_entry.symlink_metadata().is_ok()
                    && !self.overwrite_if_newer
                    && !self.overwrite_if_size_differs
                {
                    continue;
                }

                if !self.is_included(rel_dest) {
                    continue;
                }

                // File is not present: copy it in any case
                let dest_exists = dest_entry.symlink_metadata().is_ok();

                if !dest_exists {
                    debug!(
                        "Dest not present: CP {} DST {}",
                        entry.path().display(),
                        dest_entry.display()
                    );
                }

                // Conditional overwrite checks (OR semantics: copy if any enabled condition matches).
                // overwrite_all takes precedence over them.
                if !self.overwrite_all
                    && dest_exists
                    && (self.overwrite_if_newer || self.overwrite_if_size_differs)
                {
                    let newer = self.overwrite_if_newer && is_file_newer(entry.path(), &dest_entry);
                    let size_differs = self.overwrite_if_size_differs
                        && is_filesize_different(entry.path(), &dest_entry);
                    if newer {
                        debug!(
                            "Source newer: CP {} DST {}",
                            entry.path().display(),
                            dest_entry.display()
                        );
                    }
                    if size_differs {
                        debug!(
                            "Source differs: CP {} DST {}",
                            entry.path().display(),
                            dest_entry.display()
                        );
                    }
                    if !newer && !size_differs {
                        continue;
                    }
                }

                // With include filters, directories are only created once a file in them is copied.
                // They were checked for symlinks already, as directories are visited before their contents.
                if !self.include_filters.is_empty() {
                    if let Some(parent) = dest_entry.parent() {
                        if !parent.is_dir() {
                            debug!("MKDIR {}", parent.display());
                            std::fs::create_dir_all(parent).map_err(|e| {
                                context(
                                    e,
                                    format!("Could not create directory {}", parent.display()),
                                )
                            })?;
                        }
                    }
                }

                if entry.file_type().is_file() {
                    // Never write through a symlink in dest, as it may point outside of it
                    if is_symlink(&dest_entry) {
                        debug!("RM LNK {}", dest_entry.display());
                        remove_symlink(&dest_entry)?;
                    }
                    // The regular copy operation
                    debug!("CP {} DST {}", entry.path().display(), dest_entry.display());
                    copy(entry.path(), &dest_entry).map_err(|e| {
                        context(
                            e,
                            format!(
                                "Could not copy {} to {}",
                                entry.path().display(),
                                dest_entry.display()
                            ),
                        )
                    })?;
                } else {
                    debug!(
                        "CP LNK {} DST {}",
                        entry.path().display(),
                        dest_entry.display()
                    );
                    let target = read_link(entry.path()).map_err(|e| {
                        context(
                            e,
                            format!("Could not read symlink {}", entry.path().display()),
                        )
                    })?;
                    // Creating a symlink fails if dest is already present
                    if dest_exists {
                        debug!("RM {}", dest_entry.display());
                        remove_symlink(&dest_entry)?;
                    }
                    create_symlink(entry.path(), &target, &dest_entry).map_err(|e| {
                        context(
                            e,
                            format!(
                                "Could not create symlink {} to {}",
                                dest_entry.display(),
                                target.display()
                            ),
                        )
                    })?;
                }
            } else if entry.path().is_dir() {
                if is_symlink(&dest_entry) {
                    // Copying into a symlinked dir could write outside of dest
                    if !self.overwrite_all {
                        return Err(Error::new(
                            ErrorKind::AlreadyExists,
                            format!(
                                "Destination {} is a symlink, refusing to copy directory {} into it",
                                dest_entry.display(),
                                entry.path().display()
                            ),
                        ));
                    }
                    debug!("RM LNK {}", dest_entry.display());
                    remove_symlink(&dest_entry)?;
                }
                // With include filters, only matching directories are created here
                if !dest_entry.is_dir() && self.is_included(rel_dest) {
                    debug!("MKDIR {}", entry.path().display());
                    std::fs::create_dir_all(&dest_entry).map_err(|e| {
                        context(
                            e,
                            format!("Could not create directory {}", dest_entry.display()),
                        )
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Formerly executed the copy operation in parallel. Now only calls [`CopyBuilder::run`].
    #[deprecated(
        since = "0.3.21",
        note = "please use `run` instead. This is now just a wrapper around `run`."
    )]
    pub fn run_par(&self) -> Result<(), std::io::Error> {
        self.run()
    }
}

/// Copy a directory from `source` to `dest`, creating `dest`, with all options.
pub fn copy_dir_advanced<P: AsRef<Path>, Q: AsRef<Path>>(
    source: P,
    dest: Q,
    overwrite_all: bool,
    overwrite_if_newer: bool,
    overwrite_if_size_differs: bool,
    exclude_filters: Vec<String>,
    include_filters: Vec<String>,
) -> Result<(), std::io::Error> {
    CopyBuilder {
        source: source.as_ref().to_path_buf(),
        destination: dest.as_ref().to_path_buf(),
        overwrite_all,
        overwrite_if_newer,
        overwrite_if_size_differs,
        exclude_filters,
        include_filters,
        progress_callback: None,
    }
    .run()
}

/// Copy a directory from `source` to `dest`, creating `dest`, with minimal options.
pub fn copy_dir<P: AsRef<Path>, Q: AsRef<Path>>(source: P, dest: Q) -> Result<(), std::io::Error> {
    CopyBuilder {
        source: source.as_ref().to_path_buf(),
        destination: dest.as_ref().to_path_buf(),
        overwrite_all: false,
        overwrite_if_newer: false,
        overwrite_if_size_differs: false,
        exclude_filters: vec![],
        include_filters: vec![],
        progress_callback: None,
    }
    .run()
}
