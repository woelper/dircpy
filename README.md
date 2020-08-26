# dircpy

[![Actions Status](https://github.com/woelper/dircpy/workflows/Rust/badge.svg)](https://github.com/woelper/dircpy/actions)

A library to recursively copy directories, with some convenience added.


```rust
  
  // Simple example
  DirCopy::new(
      &Path::new("source"),
      &Path::new("dest"),
  )
  .build()
  .unwrap();
  
  // Copy recursively, only including certain files
  DirCopy::new(
      &Path::new("source"),
      &Path::new("dest"),
  )
  .overwrite_if_newer(true)
  .overwrite_if_size_differs(true)
  .with_include_filter(".txt")
  .with_include_filter(".csv")
  .build()
  .unwrap();
  
  
  
```
Todo:
Preserves executable bit
