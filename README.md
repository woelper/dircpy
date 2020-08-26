# dircpy

[![Actions Status](https://github.com/woelper/dircpy/workflows/Rust/badge.svg)](https://github.com/woelper/dircpy/actions)

Recursively copy directories, with some convenience added.

- Preserves permissions, also executable bit
- Option: overwriting only of modification time is newer

```rust
  DirCopy::new(
      &Path::new("source"),
      &Path::new("dest"),
  )
  .overwrite_if_newer(true)
  .build()
  .unwrap();
```
