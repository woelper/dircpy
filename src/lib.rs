use std::fs::copy;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
        .overwrite_only_newer(true)
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
    pub overwrite: bool,
    pub overwrite_only_newer: bool,
}

impl DirCopy {
    pub fn new(source: &Path, destination: &Path) -> DirCopy {
        DirCopy {
            source: source.to_owned(),
            destination: destination.to_path_buf(),
            overwrite: false,
            overwrite_only_newer: false,
        }
    }

    pub fn overwrite(self, overwrite: bool) -> DirCopy {
        DirCopy {
            overwrite,
            ..self.clone()
        }
    }

    pub fn overwrite_only_newer(self, overwrite_only_newer: bool) -> DirCopy {
        DirCopy {
            overwrite_only_newer,
            ..self.clone()
        }
    }

    pub fn build(&self) -> Result<(), std::io::Error> {
        let abs_source = self.source.canonicalize().unwrap();

        if self.destination.is_dir() {
        } else {
            std::fs::create_dir_all(&self.destination)?;
        }
        let abs_dest = self.destination.canonicalize().unwrap();

        for entry in WalkDir::new(&abs_source).into_iter().filter_map(|e| e.ok()) {
            let rel_dest = entry.path().strip_prefix(&abs_source).unwrap();
            let dest_entry = abs_dest.join(rel_dest);
            println!("SRC {} DST {:?}", abs_source.display(), dest_entry);

            if entry.path().is_file() {
                println!("CP {:?} DST {:?}", entry.path(), dest_entry);

                // Early out if target is present and overwrite is off
                if !self.overwrite && dest_entry.is_file() {
                    continue;
                }

                if self.overwrite_only_newer { //overwrite if newer
                    if entry.path().metadata().unwrap().modified().unwrap()
                        >= dest_entry.metadata().unwrap().modified().unwrap()
                    {
                        println!("FILE NEWER {:?} DST {:?}", entry.path(), dest_entry);
                        copy(entry.path(), dest_entry)?;
                    }
                } else { // overwrite all files
                    copy(entry.path(), dest_entry)?;
                }
            } else if entry.path().is_dir() {
                println!("MKDIR {:?}", entry.path());
                std::fs::create_dir_all(dest_entry)?;
                // copy(entry.path(), dest_entry).unwrap();
            }
        }

        Ok(())
    }
}
