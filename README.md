# dircpy
[![Crates.io](https://img.shields.io/crates/v/dircpy.svg)](https://crates.io/crates/dircpy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/woelper/dircpy/blob/master/LICENSE)
[![Docs Status](https://docs.rs/dircpy/badge.svg)](https://docs.rs/dircpy)

![Crates.io](https://img.shields.io/crates/d/dircpy?label=crates.io%20downloads)

[![Test Linux](https://github.com/woelper/dircpy/actions/workflows/test_linux.yml/badge.svg)](https://github.com/woelper/dircpy/actions/workflows/test_linux.yml)
[![Test Windows](https://github.com/woelper/dircpy/actions/workflows/test_windows.yml/badge.svg)](https://github.com/woelper/dircpy/actions/workflows/test_windows.yml)

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
  
```
