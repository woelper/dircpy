# dircpy

[![Actions Status](https://github.com/woelper/dircpy/workflows/Rust/badge.svg)](https://github.com/woelper/dircpy/actions)

A library to recursively copy directories, with some convenience added.


```rust
 use dircpy::*;

 // Most basid example:
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
Todo:
Preserves executable bit
