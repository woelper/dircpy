# dircpy
[![Crates.io](https://img.shields.io/crates/v/dircpy.svg)](https://crates.io/crates/dircpy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/woelper/dircpy/blob/master/LICENSE)
[![Docs Status](https://docs.rs/dircpy/badge.svg)](https://docs.rs/dircpy)
[![build](https://github.com/woelper/dircpy/actions/workflows/rust.yml/badge.svg)](https://github.com/woelper/dircpy/actions/workflows/rust.yml)
A library to recursively copy directories, with some convenience added.


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
