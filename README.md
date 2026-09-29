# dircpy
[![Crates.io](https://img.shields.io/crates/v/dircpy.svg)](https://crates.io/crates/dircpy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/woelper/dircpy/blob/master/LICENSE)
[![Docs Status](https://docs.rs/dircpy/badge.svg)](https://docs.rs/dircpy)

![Crates.io](https://img.shields.io/crates/d/dircpy?label=crates.io%20downloads)

[![Test Linux](https://github.com/woelper/dircpy/actions/workflows/test_linux.yml/badge.svg)](https://github.com/woelper/dircpy/actions/workflows/test_linux.yml)
[![Test Windows](https://github.com/woelper/dircpy/actions/workflows/test_windows.yml/badge.svg)](https://github.com/woelper/dircpy/actions/workflows/test_windows.yml)
[![MSRV](https://github.com/woelper/dircpy/actions/workflows/msrv.yml/badge.svg)](https://github.com/woelper/dircpy/actions/workflows/msrv.yml)

A cross-platform library to recursively copy directories, with some convenience added.


```rust
use dircpy::*;

// Most basic example:
copy_dir("src", "dest");

// Simple builder example:
CopyBuilder::new("src", "dest")
  .run()
  .unwrap();

// Copy recursively, only including certain files:
CopyBuilder::new("src", "dest")
  .overwrite_if_newer(true)
  .overwrite_if_size_differs(true)
  .with_include_filter(".txt")
  .with_include_filter(".csv")
  .run()
  .unwrap();

// Copy with progress:
CopyBuilder::new("src", "dest")
  .with_progress(|all, done| {
    println!("copied {done}/{all}");
  })
  .run()
  .unwrap();

// Copy files with up to 4 threads, which is faster for many small files:
CopyBuilder::new("src", "dest")
  .run_par()
  .unwrap();
```

## Performance

`run_par` walks the source on the calling thread, creating directories and doing all checks, while up to 4 threads copy the files. This is considerably faster for many small files on SSDs. For large files, copying is limited by the disk, and `run_par` has no advantage. On spinning disks, parallel copying can be slower.

Copying to a new destination, measured with `cargo bench` on Linux with a Ryzen 7 4800H and an NVMe SSD:

| Data | `run` | `run_par` | `cp -r` |
|---|---|---|---|
| 10000 files of 2 KiB | 348 ms | 146 ms | 321 ms |
| 500 files of 256 KiB | 76 ms | 32 ms | 76 ms |
| Source tree: 4525 files of 0.5 to 16 KiB in 780 directories | 167 ms | 40 ms | 156 ms |

Copying again with `overwrite_if_newer` and `overwrite_if_size_differs` only compares metadata, and takes 48 ms for the 10000 files, with either method.

Run `cargo bench` to measure on your system. It compares with [lms](https://crates.io/crates/lms) as well, if installed.
