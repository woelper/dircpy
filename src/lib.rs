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
//!
//! // Copy files with up to 4 threads, which is faster for many small files:
//!CopyBuilder::new("src", "dest")
//!.run_par()
//!.unwrap();
//! ```

use log::*;
// use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::{read_link, Metadata};
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
/// // Copy files with up to 4 threads, which is faster for many small files:
///CopyBuilder::new("src", "dest")
///.run_par()
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

/// Determine if the modification date of meta_a is newer than that of meta_b
fn is_file_newer(meta_a: &Metadata, meta_b: &Metadata) -> bool {
    meta_a.modified().unwrap_or_else(|_| SystemTime::now())
        > meta_b.modified().unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Determine if the size of meta_a and meta_b differs.
fn is_filesize_different(meta_a: &Metadata, meta_b: &Metadata) -> bool {
    meta_a.len() != meta_b.len()
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

/// Reports progress of a copy operation
struct Progress<'a> {
    callback: Option<&'a ProgressFn>,
    total: usize,
    processed: usize,
}

impl Progress<'_> {
    /// Report that an entry has been processed
    fn step(&mut self) {
        if let Some(callback) = self.callback {
            self.processed += 1;
            callback(self.total, self.processed);
        }
    }
}

/// A file operation, created while walking the source
enum Job {
    /// Copy a regular file
    Copy {
        source: PathBuf,
        dest: PathBuf,
        dest_is_new: bool,
    },
    /// Create a symlink at `link` pointing to `target`, as a copy of `original`
    Symlink {
        original: PathBuf,
        target: PathBuf,
        link: PathBuf,
    },
}

impl Job {
    fn execute(&self) -> Result<(), Error> {
        match self {
            Job::Copy {
                source,
                dest,
                dest_is_new,
            } => copy_file(source, dest, *dest_is_new).map_err(|e| {
                context(
                    e,
                    format!("Could not copy {} to {}", source.display(), dest.display()),
                )
            }),
            Job::Symlink {
                original,
                target,
                link,
            } => create_symlink(original, target, link).map_err(|e| {
                context(
                    e,
                    format!(
                        "Could not create symlink {} to {}",
                        link.display(),
                        target.display()
                    ),
                )
            }),
        }
    }
}

/// Copy a regular file, including its permissions. `dest_is_new` means nothing exists at dest.
///
/// On Linux, a new dest is created exclusively, which fails instead of following a symlink that
/// appeared in the meantime. Permissions are only set if creating dest did not already result in them,
/// which saves a syscall per file compared to `std::fs::copy`.
#[cfg(target_os = "linux")]
fn copy_file(source: &Path, dest: &Path, dest_is_new: bool) -> Result<(), Error> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut reader = std::fs::File::open(source)?;
    let permissions = reader.metadata()?.permissions();
    let mut options = std::fs::OpenOptions::new();
    options.write(true).mode(permissions.mode());
    if dest_is_new {
        options.create_new(true);
    } else {
        options.create(true).truncate(true);
    }
    let mut writer = options.open(dest)?;
    // The mode is restricted by the umask on creation, and not applied at all to existing files
    let writer_meta = writer.metadata()?;
    if writer_meta.is_file() && writer_meta.permissions().mode() != permissions.mode() {
        writer.set_permissions(permissions)?;
    }
    // Uses copy_file_range, like std::fs::copy
    std::io::copy(&mut reader, &mut writer)?;
    Ok(())
}

/// Copy a regular file, including its permissions. `dest_is_new` means nothing exists at dest.
#[cfg(not(target_os = "linux"))]
fn copy_file(source: &Path, dest: &Path, _dest_is_new: bool) -> Result<(), Error> {
    std::fs::copy(source, dest).map(|_| ())
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
        self.run_with_threads(1)
    }

    /// Execute the copy operation, copying files with up to 4 threads.
    ///
    /// The source is walked, directories are created and all checks are done on the calling thread,
    /// while files are copied in parallel. This is faster for many small files on SSDs, but does not
    /// help with large files, and can be slower on spinning disks.
    pub fn run_par(&self) -> Result<(), std::io::Error> {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .min(4);
        self.run_with_threads(threads)
    }

    /// Execute the copy operation, executing file operations on `threads` threads.
    fn run_with_threads(&self, threads: usize) -> Result<(), std::io::Error> {
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
        let dest_created = !self.destination.is_dir();
        if dest_created {
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

        // Directories created by this run. Nothing can exist in them yet, which saves looking up dest.
        // Only used on Linux, where new files are created exclusively and can't follow a symlink
        // that appeared in the meantime.
        let mut fresh_dirs: HashSet<PathBuf> = HashSet::new();
        if cfg!(target_os = "linux") && dest_created {
            fresh_dirs.insert(abs_dest.clone());
        }

        // Skip dest if it is inside source, and excluded entries including their contents
        let walk = || {
            WalkDir::new(&abs_source).into_iter().filter_entry(|e| {
                e.path() != abs_dest
                    && !self.is_excluded(e.path().strip_prefix(&abs_source).unwrap_or(e.path()))
            })
        };

        let mut progress = Progress {
            callback: self.progress_callback.as_ref(),
            total: 1,
            processed: 0,
        };
        if self.progress_callback.is_some() {
            progress.total = walk().filter_map(|e| e.ok()).count();
        }

        if threads <= 1 {
            for entry in walk() {
                // Don't ignore errors, as this would silently result in an incomplete copy
                let entry = entry?;
                if let Some(job) =
                    self.process_entry(entry, &abs_source, &abs_dest, &mut fresh_dirs)?
                {
                    job.execute()?;
                }
                progress.step();
            }
            return Ok(());
        }

        // Jobs are handed to the workers through a bounded queue, so memory use does not grow with
        // the number of files, and copying starts while the source is still being walked.
        let (job_sender, job_receiver) = std::sync::mpsc::sync_channel::<Job>(256);
        let job_receiver = std::sync::Mutex::new(job_receiver);
        let (done_sender, done_receiver) = std::sync::mpsc::channel::<Result<(), Error>>();
        std::thread::scope(|scope| {
            for _ in 0..threads {
                let job_receiver = &job_receiver;
                let done_sender = done_sender.clone();
                scope.spawn(move || loop {
                    let job = match job_receiver.lock().unwrap().recv() {
                        Ok(job) => job,
                        // All jobs are done
                        Err(_) => break,
                    };
                    // Stop once the result can't be received anymore, due to an error
                    if done_sender.send(job.execute()).is_err() {
                        break;
                    }
                });
            }
            drop(done_sender);

            // Returning early drops the job sender and done receiver, which stops the workers
            for entry in walk() {
                // Don't ignore errors, as this would silently result in an incomplete copy
                let entry = entry?;
                match self.process_entry(entry, &abs_source, &abs_dest, &mut fresh_dirs)? {
                    Some(job) => job_sender
                        .send(job)
                        .map_err(|_| Error::new(ErrorKind::Other, "Copy workers stopped"))?,
                    None => progress.step(),
                }
                // Progress is reported on this thread, as the callback may not be thread-safe
                for result in done_receiver.try_iter() {
                    result?;
                    progress.step();
                }
            }
            drop(job_sender);
            for result in done_receiver.iter() {
                result?;
                progress.step();
            }
            Ok(())
        })
    }

    /// Check a source entry, prepare dest for it, and return the file operation to execute, if any.
    /// Directories are created right away, as their contents are visited after them.
    fn process_entry(
        &self,
        entry: walkdir::DirEntry,
        abs_source: &Path,
        abs_dest: &Path,
        fresh_dirs: &mut HashSet<PathBuf>,
    ) -> Result<Option<Job>, std::io::Error> {
        let rel_dest = entry.path().strip_prefix(abs_source).map_err(|e| {
            Error::new(ErrorKind::Other, format!("Could not strip prefix: {:?}", e))
        })?;
        let dest_entry = abs_dest.join(rel_dest);
        let in_fresh_dir = !fresh_dirs.is_empty()
            && dest_entry
                .parent()
                .map_or(false, |parent| fresh_dirs.contains(parent));

        if !entry.file_type().is_dir() {
            if !entry.file_type().is_file() && !entry.file_type().is_symlink() {
                // Sockets, fifos and devices can't be copied meaningfully
                warn!(
                    "Skipping {}: unsupported file type {:?}",
                    entry.path().display(),
                    entry.file_type()
                );
                return Ok(None);
            }

            if !self.is_included(rel_dest) {
                return Ok(None);
            }

            // Look up dest only once, the result is used for all checks below
            let dest_meta = if in_fresh_dir {
                None
            } else {
                dest_entry.symlink_metadata().ok()
            };
            let dest_exists = dest_meta.is_some();

            // Early out if target is present and overwrite is off
            if !self.overwrite_all
                && dest_exists
                && !self.overwrite_if_newer
                && !self.overwrite_if_size_differs
            {
                return Ok(None);
            }

            // File is not present: copy it in any case
            if !dest_exists {
                debug!(
                    "Dest not present: CP {} DST {}",
                    entry.path().display(),
                    dest_entry.display()
                );
            }

            // Conditional overwrite checks (OR semantics: copy if any enabled condition matches).
            // overwrite_all takes precedence over them.
            if let (false, Some(dest_meta), true) = (
                self.overwrite_all,
                &dest_meta,
                self.overwrite_if_newer || self.overwrite_if_size_differs,
            ) {
                let (newer, size_differs) = match entry.metadata() {
                    Ok(source_meta) => (
                        self.overwrite_if_newer && is_file_newer(&source_meta, dest_meta),
                        self.overwrite_if_size_differs
                            && is_filesize_different(&source_meta, dest_meta),
                    ),
                    Err(_) => (false, false),
                };
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
                    return Ok(None);
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
                let mut dest_is_new = !dest_exists;
                // Never write through a symlink in dest, as it may point outside of it
                if dest_meta.map_or(false, |m| m.file_type().is_symlink()) {
                    debug!("RM LNK {}", dest_entry.display());
                    remove_symlink(&dest_entry)?;
                    dest_is_new = true;
                }
                // The regular copy operation
                debug!("CP {} DST {}", entry.path().display(), dest_entry.display());
                return Ok(Some(Job::Copy {
                    source: entry.into_path(),
                    dest: dest_entry,
                    dest_is_new,
                }));
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
                return Ok(Some(Job::Symlink {
                    original: entry.into_path(),
                    target,
                    link: dest_entry,
                }));
            }
        } else {
            let mut dest_meta = if in_fresh_dir {
                None
            } else {
                dest_entry.symlink_metadata().ok()
            };
            if dest_meta
                .as_ref()
                .map_or(false, |m| m.file_type().is_symlink())
            {
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
                dest_meta = None;
            }
            // With include filters, only matching directories are created here
            if !dest_meta.map_or(false, |m| m.is_dir()) && self.is_included(rel_dest) {
                debug!("MKDIR {}", entry.path().display());
                std::fs::create_dir_all(&dest_entry).map_err(|e| {
                    context(
                        e,
                        format!("Could not create directory {}", dest_entry.display()),
                    )
                })?;
                if cfg!(target_os = "linux") {
                    fresh_dirs.insert(dest_entry);
                }
            }
        }
        Ok(None)
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
