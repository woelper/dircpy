0.3.21:
Fix: files were not copied for some combinations of `overwrite_if_newer` and `overwrite_if_size_differs`. Conditions are now OR-combined
Fix: `overwrite(true)` was ignored when conditional overwrite options were also set
Fix: overwriting could write through symlinks in the destination to locations outside of it
Fix: copying over an existing symlink failed with "File exists"
Fix: special files (sockets, fifos, devices) caused a panic. They are now skipped with a warning
Fix: a missing source directory still created the destination
Fix: errors while reading the source (e.g. permission denied) were ignored, resulting in an incomplete copy. They are now returned
Fix: include and exclude filters also matched the source path itself. They now only match the path relative to the source
Fix: a file as source did nothing and returned Ok. It now returns an error
Fix: symlinks were silently not copied on Windows. Creating them requires developer mode or admin rights, otherwise an error is returned
Fix: filters did not apply to directories. Excluded directories are skipped entirely. With include filters, only directories that match or contain copied files are created
Errors now contain the affected paths
Copying a directory onto a symlink in the destination now returns an error unless `overwrite(true)` is set
`run_par` copies files with up to 4 threads while walking the source, without jwalk. About 2x faster than `run` for many small files
Remove the jwalk dependency. The `jwalk` feature is kept for compatibility, but has no effect
Faster copying through fewer metadata lookups per file: re-runs with `overwrite_if_newer` or `overwrite_if_size_differs` are about 2x faster. On Linux, copying many small files is about 1.2x faster
On Linux, new files are created exclusively, so a symlink appearing in the destination during the copy is never written through
Switch to edition 2021 and declare the minimum Rust version (1.64), checked in CI
Fix wrong documentation for filters, progress callback and `overwrite_if_newer`
More tests
0.3.19:
Fix for empty include lists (Thanks @AdamLeyshon)
Add test for empty include lists
0.3.18:
Update walkdir
0.3.17:
Fixed bug where multiple include patterns would not work (Thanks @giovannimirarchi420)
