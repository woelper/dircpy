use std::fs::copy;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use log::*;
use std::time::SystemTime;
use std::io::{Error, ErrorKind};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::create_dir_all;

    #[test]
    fn copy() {
        let sourcepath = "source/level1/level2/level3";
        create_dir_all(sourcepath).unwrap();
        std::fs::write("source/test", vec![]).unwrap();
        std::fs::write("source/level1/other_file", vec![]).unwrap();

        DirCopy::new(
            &Path::new("source").canonicalize().unwrap(),
            &Path::new("dest"),
        )
        .overwrite(true)
        .overwrite_if_newer(true)
        .build()
        .unwrap();

        // clean up
        std::fs::remove_dir_all("source").unwrap();
        std::fs::remove_dir_all("dest").unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct DirCopy {
    pub source: PathBuf,
    pub destination: PathBuf,
    overwrite_all: bool,
    overwrite_if_newer: bool,
    overwrite_if_size_differs: bool,
    exclude_filters: Vec<String>,
    include_filters: Vec<String>
}

/// Determine if the modification date of file_a is newer than that of file_b
fn is_file_newer(file_a: &Path, file_b: &Path) -> bool {
    match (file_a.metadata(), file_b.metadata()) {
        (Ok(meta_a), Ok(meta_b)) => {
            meta_a.modified().unwrap_or(SystemTime::now()) > meta_b.modified().unwrap_or(SystemTime::UNIX_EPOCH) 
        },
        _ => false
    }
}

/// Determine if file_a and file_b's size differs. 
fn is_filesize_different(file_a: &Path, file_b: &Path) -> bool {
    match (file_a.metadata(), file_b.metadata()) {
        (Ok(meta_a), Ok(meta_b)) => {
            meta_a.len() != meta_b.len() 
        },
        _ => false
    }
}


impl DirCopy {
    pub fn new(source: &Path, destination: &Path) -> DirCopy {
        DirCopy {
            source: source.to_owned(),
            destination: destination.to_path_buf(),
            overwrite_all: false,
            overwrite_if_newer: false,
            overwrite_if_size_differs: false,
            exclude_filters: vec![],
            include_filters: vec![]
        }
    }

    pub fn overwrite(self, overwrite: bool) -> DirCopy {
        DirCopy {
            overwrite_all: overwrite,
            ..self.clone()
        }
    }

    pub fn overwrite_if_newer(self, overwrite_only_newer: bool) -> DirCopy {
        DirCopy {
            overwrite_if_newer: overwrite_only_newer,
            ..self.clone()
        }
    }

    pub fn overwrite_if_size_differs(self, overwrite_if_size_differs: bool) -> DirCopy {
        DirCopy {
            overwrite_if_size_differs: overwrite_if_size_differs,
            ..self.clone()
        }
    }

    /// Do not copy files that contain this string
    pub fn with_exclude_filter(self, f: &str) -> DirCopy {
        let mut filters = self.exclude_filters.clone();
        filters.push(f.to_owned());
        DirCopy {
            exclude_filters: filters,
            ..self.clone()
        }
    }
    
    /// Only copy files that contain this string.
    pub fn with_include_filter(self, f: &str) -> DirCopy {
        let mut filters = self.exclude_filters.clone();
        filters.push(f.to_owned());
        DirCopy {
            include_filters: filters,
            ..self.clone()
        }
    }

    pub fn build(&self) -> Result<(), std::io::Error> {
        
        if !self.destination.is_dir() {
            debug!("MKDIR {:?}", &self.destination);
            std::fs::create_dir_all(&self.destination)?;
        }
        let abs_source = self.source.canonicalize()?;
        let abs_dest = self.destination.canonicalize()?;
        debug!("Building copy operation: SRC {} DST {}", abs_source.display(), abs_dest.display());

        for entry in WalkDir::new(&abs_source).into_iter().filter_map(|e| e.ok()) {
            let rel_dest = entry.path().strip_prefix(&abs_source).map_err(|e | Error::new(ErrorKind::Other, format!("Could not strip prefix: {:?}",e)) )?;
            let dest_entry = abs_dest.join(rel_dest);

            if entry.path().is_file() { // the source exists

                
                // Early out if target is present and overwrite is off
                if !self.overwrite_all && dest_entry.is_file() && !self.overwrite_if_newer && !self.overwrite_if_size_differs {
                    continue;
                }

                for f in &self.exclude_filters {
                    if entry.path().to_string_lossy().contains(f) {
                        continue;
                    }
                }
                
                for f in &self.include_filters {
                    if !entry.path().to_string_lossy().contains(f) {
                        continue;
                     }
                }

                // File is not present: copy it
                if !dest_entry.is_file() {
                    debug!("Dest not present: CP {} DST {}", entry.path().display(), dest_entry.display());
                    copy(entry.path(), dest_entry)?;
                    continue;
                }

                // File newer?
                if self.overwrite_if_newer { 
                    if is_file_newer(entry.path(), &dest_entry) {
                        debug!("Source newer: CP {} DST {}", entry.path().display(), dest_entry.display());
                        copy(entry.path(), &dest_entry)?;
                    }
                    continue;
                }

                // Different size?
                if self.overwrite_if_size_differs {
                    if is_filesize_different(entry.path(), &dest_entry) {
                        debug!("Source differs: CP {} DST {}", entry.path().display(), dest_entry.display());
                        copy(entry.path(), &dest_entry)?;
                    }
                    continue;
                }
                     
                
                // The regular copy operation
                debug!("CP {} DST {}", entry.path().display(), dest_entry.display());
                copy(entry.path(), dest_entry)?;
            
            
            } else if entry.path().is_dir() && !dest_entry.is_dir() {
                debug!("MKDIR {}", entry.path().display());
                std::fs::create_dir_all(dest_entry)?;
            }
        }

        Ok(())
    }
}
