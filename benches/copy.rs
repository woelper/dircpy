//! Benchmarks `run` and `run_par` against `cp -r` (and `lms`, if installed) on generated data.
//! Run with `cargo bench`. The data is generated once into `bench_data`.

use criterion::*;
use dircpy::CopyBuilder;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const DATA: &str = "bench_data";

/// A generated source directory
struct Dataset {
    name: &'static str,
    /// Number of subdirectories per directory, for each level of nesting
    dirs_per_level: usize,
    /// Number of files per directory and their size, for each level of nesting
    levels: &'static [(usize, usize)],
}

const DATASETS: &[Dataset] = &[
    // Many small files, like a cache or a mail directory: 10000 files of 2 KiB
    Dataset {
        name: "small_files",
        dirs_per_level: 100,
        levels: &[(100, 2 * 1024)],
    },
    // Fewer, larger files, like photos: 500 files of 256 KiB
    Dataset {
        name: "medium_files",
        dirs_per_level: 10,
        levels: &[(50, 256 * 1024)],
    },
    // A deep tree of mixed sizes, like source code: 4525 files in 780 directories
    Dataset {
        name: "source_tree",
        dirs_per_level: 5,
        levels: &[(5, 16 * 1024), (5, 4 * 1024), (10, 1024), (5, 512)],
    },
];

impl Dataset {
    fn path(&self) -> PathBuf {
        Path::new(DATA).join("source").join(self.name)
    }

    /// Generate the dataset if not present yet, and return the number of files in it.
    fn generate(&self) -> u64 {
        let root = self.path();
        let marker = root.join(".complete");
        let generated = marker.exists();
        let mut files = 0;
        let mut dirs = vec![root];
        for (level, (files_per_dir, size)) in self.levels.iter().enumerate() {
            let mut subdirs = vec![];
            for dir in &dirs {
                for d in 0..self.dirs_per_level {
                    let subdir = dir.join(format!("dir{level}_{d}"));
                    if !generated {
                        std::fs::create_dir_all(&subdir).unwrap();
                        for f in 0..*files_per_dir {
                            let content: Vec<u8> = (0..*size).map(|i| (i * 31 + f) as u8).collect();
                            std::fs::write(subdir.join(format!("file{f}")), content).unwrap();
                        }
                    }
                    files += *files_per_dir as u64;
                    subdirs.push(subdir);
                }
            }
            dirs = subdirs;
        }
        std::fs::write(&marker, "").unwrap();
        files
    }
}

/// A destination path, which does not exist yet
fn new_dest(name: &str) -> PathBuf {
    let dest = Path::new(DATA).join("dest").join(name);
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    dest
}

/// Time only the copy operation, removing the copy after each iteration.
fn time_copies(iters: u64, name: &str, copy: impl Fn(&Path)) -> Duration {
    let mut total = Duration::ZERO;
    for _ in 0..iters {
        let dest = new_dest(name);
        let start = Instant::now();
        copy(&dest);
        total += start.elapsed();
        std::fs::remove_dir_all(&dest).unwrap();
    }
    total
}

/// Determine if lms is installed. Other tools are named lms as well, so try to copy with it.
fn has_lms() -> bool {
    let source = Path::new(DATA).join("lms_check");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("file"), "").unwrap();
    let dest = new_dest("lms_check");
    let copied = Command::new("lms")
        .arg("cp")
        .arg(&source)
        .arg(&dest)
        .output()
        .map_or(false, |output| output.status.success())
        && dest.join("file").is_file();
    let _ = std::fs::remove_dir_all(&source);
    let _ = std::fs::remove_dir_all(&dest);
    copied
}

fn run_command(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{:?} failed: {:?}",
        command,
        output
    );
}

/// Copying to a new destination
fn bench_copy(c: &mut Criterion) {
    // Not all cp implementations support --help, so only check if it can be started
    let has_cp = cfg!(unix) && Command::new("cp").arg("--help").output().is_ok();
    let has_lms = has_lms();
    for dataset in DATASETS {
        let files = dataset.generate();
        let source = dataset.path();
        let mut group = c.benchmark_group(format!("copy/{}", dataset.name));
        group.throughput(Throughput::Elements(files));

        group.bench_function("run", |b| {
            b.iter_custom(|iters| {
                time_copies(iters, "run", |dest| {
                    CopyBuilder::new(&source, dest).run().unwrap()
                })
            })
        });
        group.bench_function("run_par", |b| {
            b.iter_custom(|iters| {
                time_copies(iters, "run_par", |dest| {
                    CopyBuilder::new(&source, dest).run_par().unwrap()
                })
            })
        });
        if has_cp {
            group.bench_function("cp -r", |b| {
                b.iter_custom(|iters| {
                    time_copies(iters, "cp", |dest| {
                        run_command(Command::new("cp").arg("-r").arg(&source).arg(dest))
                    })
                })
            });
        }
        if has_lms {
            group.bench_function("lms cp", |b| {
                b.iter_custom(|iters| {
                    time_copies(iters, "lms", |dest| {
                        run_command(Command::new("lms").arg("cp").arg(&source).arg(dest))
                    })
                })
            });
        }
        group.finish();
    }
}

/// Copying to an existing copy where nothing changed, with conditional overwrites
fn bench_resync(c: &mut Criterion) {
    for dataset in DATASETS {
        let files = dataset.generate();
        let source = dataset.path();
        let dest = new_dest("resync");
        CopyBuilder::new(&source, &dest).run().unwrap();
        let builder = CopyBuilder::new(&source, &dest)
            .overwrite_if_newer(true)
            .overwrite_if_size_differs(true);

        let mut group = c.benchmark_group(format!("resync/{}", dataset.name));
        group.throughput(Throughput::Elements(files));
        group.bench_function("run", |b| b.iter(|| builder.run().unwrap()));
        group.bench_function("run_par", |b| b.iter(|| builder.run_par().unwrap()));
        group.finish();
        std::fs::remove_dir_all(&dest).unwrap();
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(6));
    targets = bench_copy, bench_resync
}
criterion_main!(benches);
