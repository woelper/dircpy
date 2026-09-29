use super::*;
use std::fs::create_dir_all;
use std::fs::File;
use std::io::read_to_string;
#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};

#[test]
fn copy_basic() {
    std::env::set_var("RUST_LOG", "DEBUG");
    let _ = env_logger::builder().try_init();

    let src = "basic_src";
    let dst = "basic_dest";

    create_dir_all(format!("{src}/level1/level2/level3")).unwrap();

    File::create(format!("{src}/test")).unwrap();
    File::create(format!("{src}/level1/other_file")).unwrap();

    #[cfg(unix)]
    {
        File::create(format!("{src}/exec_file")).unwrap();
        std::fs::set_permissions(
            format!("{src}/exec_file"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        symlink("exec_file", format!("{src}/symlink")).unwrap();
        symlink("does_not_exist", format!("{src}/dangling_symlink")).unwrap();
    }

    CopyBuilder::new(src, dst)
        .overwrite(true)
        .overwrite_if_newer(true)
        .run()
        .unwrap();

    #[cfg(unix)]
    {
        let f = File::open(format!("{dst}/exec_file")).unwrap();
        let metadata = f.metadata().unwrap();
        let permissions = metadata.permissions();
        println!("permissions: {:o}", permissions.mode());
        assert_eq!(permissions.mode(), 33261);
        assert_eq!(
            Path::new("exec_file"),
            read_link(format!("{dst}/symlink")).unwrap().as_path()
        );
        assert_eq!(
            Path::new("does_not_exist"),
            read_link(format!("{dst}/dangling_symlink"))
                .unwrap()
                .as_path()
        );
    }

    // clean up
    std::fs::remove_dir_all(src).unwrap();
    std::fs::remove_dir_all(dst).unwrap();
}

#[test]
fn copy_subdir() {
    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();
    create_dir_all("source/subdir").unwrap();
    create_dir_all("source/this_should_copy").unwrap();
    File::create("source/this_should_copy/file.doc").unwrap();
    File::create("source/a.jpg").unwrap();
    File::create("source/b.jpg").unwrap();
    File::create("source/d.txt").unwrap();

    CopyBuilder::new("source", "source/subdir").run().unwrap();

    std::fs::remove_dir_all("source").unwrap();
}

#[test]
fn copy_overwrite() {
    use std::fs::File;
    use std::io::Write;

    let source_dir = "overwrite_source";
    let dest_dir = "overwrite_dest";

    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();
    create_dir_all(source_dir).unwrap();
    create_dir_all(dest_dir).unwrap();
    File::create(format!("{source_dir}/a.txt")).unwrap();
    let mut file_b = File::create(format!("{source_dir}/b.txt")).unwrap();

    let contents = "Contents changed";
    // Copy once, both files are empty
    CopyBuilder::new(source_dir, dest_dir).run().unwrap();
    // write something to file b so we can check if we overwrite it
    write!(file_b, "{contents}").unwrap();
    // perform a second copy
    CopyBuilder::new(source_dir, dest_dir)
        .overwrite(true)
        .run()
        .unwrap();
    // make sure the contents of b are now changed
    let s = read_to_string(File::open(format!("{dest_dir}/b.txt")).unwrap()).unwrap();
    assert!(s == contents, "Destination was not overwritten");

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
}

#[test]
fn copy_exclude() {
    std::env::set_var("RUST_LOG", "DEBUG");
    let _ = env_logger::builder().try_init();

    let src = "ex_src";
    let dst = "ex_dest";

    create_dir_all(src).unwrap();
    File::create(format!("{src}/foo")).unwrap();
    File::create(format!("{src}/bar")).unwrap();

    CopyBuilder::new(src, dst)
        .overwrite(true)
        .overwrite_if_newer(true)
        .with_exclude_filter("foo")
        .run()
        .unwrap();

    assert!(!Path::new(&format!("{}/foo", dst)).is_file());

    // clean up
    std::fs::remove_dir_all(src).unwrap();
    std::fs::remove_dir_all(dst).unwrap();
}

#[test]
fn copy_include() {
    std::env::set_var("RUST_LOG", "DEBUG");
    let _ = env_logger::builder().try_init();

    let src = "in_src";
    let dst = "in_dest";

    create_dir_all(src).unwrap();
    File::create(format!("{src}/foo")).unwrap();
    File::create(format!("{src}/bar")).unwrap();
    File::create(format!("{src}/baz")).unwrap();

    CopyBuilder::new(src, dst)
        .overwrite(true)
        .overwrite_if_newer(true)
        .with_include_filter("foo")
        .with_include_filter("baz")
        .run()
        .unwrap();

    assert!(Path::new(&format!("{dst}/foo")).is_file());
    assert!(!Path::new(&format!("{dst}/bar")).exists());
    assert!(Path::new(&format!("{dst}/baz")).exists());

    // clean up
    std::fs::remove_dir_all(src).unwrap();
    std::fs::remove_dir_all(dst).unwrap();
}

#[test]
fn copy_empty_include() {
    std::env::set_var("RUST_LOG", "DEBUG");
    let _ = env_logger::builder().try_init();

    let src = "in_src_inc";
    let dst = "in_dest_inc";

    create_dir_all(src).unwrap();
    File::create(format!("{src}/foo")).unwrap();
    File::create(format!("{src}/bar")).unwrap();
    File::create(format!("{src}/baz")).unwrap();

    CopyBuilder::new(src, dst)
        .overwrite(true)
        .overwrite_if_newer(true)
        .run()
        .unwrap();

    assert!(Path::new(&format!("{dst}/foo")).is_file());
    assert!(Path::new(&format!("{dst}/bar")).exists());
    assert!(Path::new(&format!("{dst}/baz")).exists());

    // clean up
    std::fs::remove_dir_all(src).unwrap();
    std::fs::remove_dir_all(dst).unwrap();
}

#[test]
fn copy_cargo() {
    std::env::set_var("RUST_LOG", "DEBUG");
    let _ = env_logger::builder().try_init();
    let url = "https://github.com/rust-lang/cargo/archive/master.zip";
    let sample_dir = "cargo";
    let output_dir = format!("{sample_dir}_output");
    let archive = format!("{sample_dir}.zip");
    info!("Expanding {archive}");

    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut out = File::create(&archive).expect("failed to create file");
    std::io::copy(&mut resp, &mut out).expect("failed to copy content");

    let reader = std::fs::File::open(&archive).unwrap();

    unzip::Unzipper::new(reader, sample_dir)
        .unzip()
        .expect("Could not expand cargo sources");
    let num_input_files = WalkDir::new(sample_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .count();

    CopyBuilder::new(
        Path::new(sample_dir).canonicalize().unwrap(),
        PathBuf::from(&output_dir),
    )
    .run()
    .unwrap();

    let num_output_files = WalkDir::new(&output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .count();

    assert_eq!(num_output_files, num_input_files);

    std::fs::remove_dir_all(sample_dir).unwrap();
    std::fs::remove_dir_all(output_dir).unwrap();
    std::fs::remove_file(archive).unwrap();
}

#[test]
fn copy_cargo_progress() {
    std::env::set_var("RUST_LOG", "INFO");
    let _ = env_logger::builder().try_init();

    let url = "https://github.com/rust-lang/cargo/archive/master.zip";
    let sample_dir = "cargo_progress";
    let output_dir = format!("{sample_dir}_output");
    let archive = format!("{sample_dir}.zip");
    info!("Expanding {archive}");

    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut out = File::create(&archive).expect("failed to create file");
    std::io::copy(&mut resp, &mut out).expect("failed to copy content");

    let reader = std::fs::File::open(&archive).unwrap();

    unzip::Unzipper::new(reader, &sample_dir)
        .unzip()
        .expect("Could not expand cargo sources");
    let num_input_files = WalkDir::new(sample_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .count();

    CopyBuilder::new(
        Path::new(&sample_dir).canonicalize().unwrap(),
        PathBuf::from(&output_dir),
    )
    .with_progress(|all, done| {
        info!("copied {done}/{all}");
    })
    .run()
    .unwrap();

    let num_output_files = WalkDir::new(&output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .count();

    assert_eq!(num_output_files, num_input_files);

    std::fs::remove_dir_all(sample_dir).unwrap();
    std::fs::remove_dir_all(output_dir).unwrap();
    std::fs::remove_file(archive).unwrap();
}

#[test]
/// Source is NOT newer, but sizes differ → should copy even though source is older.
fn overwrite_or_size_differs_not_newer() {
    let sample_dir = "overwrite_size_diff";
    use std::fs::File;
    use std::io::Write;
    std::env::set_var("RUST_LOG", "DEBUG");
    let _ = env_logger::builder().try_init();

    let source_dir = format!("{sample_dir}_src");
    let dest_dir = format!("{sample_dir}_dest");

    create_dir_all(&source_dir).unwrap();
    create_dir_all(&dest_dir).unwrap();

    // Write source first so it is older.
    File::create(format!("{source_dir}/file.txt")).unwrap();

    // Small delay so the dest mtime is strictly newer.
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Write dest after with different (larger) content, making it newer AND bigger.
    let mut f = File::create(format!("{dest_dir}/file.txt")).unwrap();
    write!(f, "much longer destination content").unwrap();
    drop(f);

    // Source is older but smaller; sizes differ, so should be copied.
    CopyBuilder::new(&source_dir, &dest_dir)
        .overwrite_if_newer(true)
        .overwrite_if_size_differs(true)
        .run()
        .unwrap();

    let result = std::fs::read_to_string(format!("{dest_dir}/file.txt")).unwrap();
    assert_eq!(result, "", "File should be identical to source");

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
}

#[test]
fn overwrite_or_newer_same_size() {
    // Source IS newer, same size → should copy even though size is unchanged.
    use std::fs::File;
    use std::io::Write;
    let sample_dir = "overwrite_or_newer_same_size";

    let source_dir = format!("{sample_dir}_src");
    let dest_dir = format!("{sample_dir}_dest");

    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();
    create_dir_all(&source_dir).unwrap();
    create_dir_all(&dest_dir).unwrap();

    // Write dest first so it is older.
    let mut f = File::create(format!("{dest_dir}/file.txt")).unwrap();
    write!(f, "hello123").unwrap(); // 8 bytes
    drop(f);

    // Small delay so the source mtime is strictly newer.
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Write source after with the same length (same size, but newer).
    let source_content = "world456"; // 8 bytes
    let mut f = File::create(format!("{source_dir}/file.txt")).unwrap();
    write!(f, "{source_content}").unwrap();
    drop(f);

    CopyBuilder::new(&source_dir, &dest_dir)
        .overwrite_if_newer(true)
        .overwrite_if_size_differs(true)
        .run()
        .unwrap();

    let result = std::fs::read_to_string(format!("{dest_dir}/file.txt")).unwrap();
    assert_eq!(
        result, source_content,
        "File should be overwritten because source is newer (even though size is the same)"
    );

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
}

#[cfg(unix)]
#[test]
/// A symlink already present in dest must be replaced, not written through.
fn overwrite_does_not_follow_dest_symlink() {
    let _ = env_logger::builder().try_init();
    let source_dir = "no_follow_src";
    let dest_dir = "no_follow_dest";
    let outside_dir = "no_follow_outside";

    create_dir_all(source_dir).unwrap();
    create_dir_all(dest_dir).unwrap();
    create_dir_all(format!("{source_dir}/sub")).unwrap();
    create_dir_all(outside_dir).unwrap();
    let outside_abs = Path::new(outside_dir).canonicalize().unwrap();

    std::fs::write(format!("{source_dir}/file"), "source").unwrap();
    std::fs::write(format!("{source_dir}/sub/nested"), "source").unwrap();
    std::fs::write(format!("{outside_dir}/file"), "outside").unwrap();

    // dest/file points at a file outside of dest, dest/sub at a directory outside of dest
    symlink(outside_abs.join("file"), format!("{dest_dir}/file")).unwrap();
    symlink(&outside_abs, format!("{dest_dir}/sub")).unwrap();

    CopyBuilder::new(source_dir, dest_dir)
        .overwrite(true)
        .run()
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(format!("{outside_dir}/file")).unwrap(),
        "outside",
        "File outside of dest was overwritten through a symlink"
    );
    assert!(
        !Path::new(&format!("{outside_dir}/nested")).exists(),
        "File was copied into a directory outside of dest through a symlink"
    );
    assert!(Path::new(&format!("{dest_dir}/file"))
        .symlink_metadata()
        .unwrap()
        .is_file());
    assert_eq!(
        std::fs::read_to_string(format!("{dest_dir}/file")).unwrap(),
        "source"
    );
    assert!(Path::new(&format!("{dest_dir}/sub"))
        .symlink_metadata()
        .unwrap()
        .is_dir());
    assert_eq!(
        std::fs::read_to_string(format!("{dest_dir}/sub/nested")).unwrap(),
        "source"
    );

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
    std::fs::remove_dir_all(outside_dir).unwrap();
}

#[cfg(unix)]
#[test]
/// Copying a symlink over an existing one must replace it instead of failing.
fn overwrite_existing_symlink() {
    let _ = env_logger::builder().try_init();
    let source_dir = "overwrite_symlink_src";
    let dest_dir = "overwrite_symlink_dest";

    create_dir_all(source_dir).unwrap();
    symlink("old_target", format!("{source_dir}/link")).unwrap();

    CopyBuilder::new(source_dir, dest_dir).run().unwrap();

    std::fs::remove_file(format!("{source_dir}/link")).unwrap();
    symlink("new_target", format!("{source_dir}/link")).unwrap();

    CopyBuilder::new(source_dir, dest_dir)
        .overwrite(true)
        .run()
        .unwrap();
    assert_eq!(
        read_link(format!("{dest_dir}/link")).unwrap(),
        Path::new("new_target")
    );

    // conditional overwrite must work as well: "old_target_x" differs in size
    std::fs::remove_file(format!("{source_dir}/link")).unwrap();
    symlink("old_target_x", format!("{source_dir}/link")).unwrap();

    CopyBuilder::new(source_dir, dest_dir)
        .overwrite_if_size_differs(true)
        .run()
        .unwrap();
    assert_eq!(
        read_link(format!("{dest_dir}/link")).unwrap(),
        Path::new("old_target_x")
    );

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
}

#[test]
/// `overwrite(true)` must win even if a conditional overwrite would not copy.
fn overwrite_all_with_conditional_flags() {
    let _ = env_logger::builder().try_init();
    let source_dir = "overwrite_all_cond_src";
    let dest_dir = "overwrite_all_cond_dest";

    create_dir_all(source_dir).unwrap();
    create_dir_all(dest_dir).unwrap();

    // same size, dest is newer: neither condition matches
    std::fs::write(format!("{source_dir}/file"), "SRC").unwrap();
    std::fs::write(format!("{dest_dir}/file"), "DST").unwrap();
    let now = SystemTime::now();
    File::options()
        .write(true)
        .open(format!("{source_dir}/file"))
        .unwrap()
        .set_modified(now - std::time::Duration::from_secs(3600))
        .unwrap();
    File::options()
        .write(true)
        .open(format!("{dest_dir}/file"))
        .unwrap()
        .set_modified(now)
        .unwrap();

    CopyBuilder::new(source_dir, dest_dir)
        .overwrite(true)
        .overwrite_if_newer(true)
        .overwrite_if_size_differs(true)
        .run()
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(format!("{dest_dir}/file")).unwrap(),
        "SRC",
        "overwrite(true) was ignored because conditional flags were set"
    );

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
}

#[cfg(unix)]
#[test]
/// Special files (sockets, fifos, devices) must be skipped instead of panicking.
fn skip_special_files() {
    let _ = env_logger::builder().try_init();
    let source_dir = "special_src";
    let dest_dir = "special_dest";

    create_dir_all(source_dir).unwrap();
    File::create(format!("{source_dir}/file")).unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(format!("{source_dir}/socket")).unwrap();

    let result = CopyBuilder::new(source_dir, dest_dir).run();

    assert!(result.is_ok(), "Copy failed: {:?}", result);
    assert!(Path::new(&format!("{dest_dir}/file")).is_file());
    assert!(Path::new(&format!("{dest_dir}/socket"))
        .symlink_metadata()
        .is_err());

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
}

#[test]
/// A missing source must fail without creating the destination.
fn missing_source_does_not_create_dest() {
    let _ = env_logger::builder().try_init();
    let source_dir = "missing_src";
    let dest_dir = "missing_src_dest";

    let result = CopyBuilder::new(source_dir, dest_dir).run();

    let dest_created = Path::new(dest_dir).exists();
    let _ = std::fs::remove_dir_all(dest_dir);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::NotFound);
    assert!(
        !dest_created,
        "Destination was created for a missing source"
    );
}

#[cfg(unix)]
#[test]
/// Errors while walking the source must be returned instead of being ignored.
fn unreadable_source_dir_fails() {
    let _ = env_logger::builder().try_init();
    let source_dir = "unreadable_src";
    let dest_dir = "unreadable_dest";
    let locked = format!("{source_dir}/locked");

    create_dir_all(&locked).unwrap();
    File::create(format!("{locked}/file")).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

    // Permissions are not enforced for root, so there is nothing to test
    let is_root = std::fs::read_dir(&locked).is_ok();
    let result = CopyBuilder::new(source_dir, dest_dir).run();

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::remove_dir_all(source_dir).unwrap();
    let _ = std::fs::remove_dir_all(dest_dir);

    if is_root {
        eprintln!("Skipping unreadable_source_dir_fails: running as root");
        return;
    }
    assert_eq!(result.unwrap_err().kind(), ErrorKind::PermissionDenied);
}

#[test]
/// Filters must only match the path relative to the source, not the source itself.
fn filters_match_relative_path() {
    let _ = env_logger::builder().try_init();
    let source_dir = "rel_filter_src";
    let dest_exclude = "rel_filter_exclude_dest";
    let dest_include = "rel_filter_include_dest";

    create_dir_all(format!("{source_dir}/sub")).unwrap();
    File::create(format!("{source_dir}/a.txt")).unwrap();
    File::create(format!("{source_dir}/sub/b.txt")).unwrap();

    // "rel_filter" is only part of the source path, so nothing is excluded
    CopyBuilder::new(source_dir, dest_exclude)
        .with_exclude_filter("rel_filter")
        .run()
        .unwrap();
    // ...and nothing is included
    CopyBuilder::new(source_dir, dest_include)
        .with_include_filter("rel_filter")
        .run()
        .unwrap();

    let excluded_a = Path::new(&format!("{dest_exclude}/a.txt")).is_file();
    let excluded_b = Path::new(&format!("{dest_exclude}/sub/b.txt")).is_file();
    let included_a = Path::new(&format!("{dest_include}/a.txt")).exists();
    let included_b = Path::new(&format!("{dest_include}/sub/b.txt")).exists();

    // Directory parts of the relative path still match
    let dest_sub = "rel_filter_sub_dest";
    CopyBuilder::new(source_dir, dest_sub)
        .with_include_filter("sub")
        .run()
        .unwrap();
    let sub_a = Path::new(&format!("{dest_sub}/a.txt")).exists();
    let sub_b = Path::new(&format!("{dest_sub}/sub/b.txt")).is_file();

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_exclude).unwrap();
    std::fs::remove_dir_all(dest_include).unwrap();
    std::fs::remove_dir_all(dest_sub).unwrap();

    assert!(
        excluded_a && excluded_b,
        "Exclude filter matched the source path"
    );
    assert!(
        !included_a && !included_b,
        "Include filter matched the source path"
    );
    assert!(
        !sub_a && sub_b,
        "Include filter did not match a relative directory"
    );
}

#[test]
/// A file as source must fail without creating the destination.
fn file_source_fails() {
    let _ = env_logger::builder().try_init();
    let source_file = "file_source";
    let dest_dir = "file_source_dest";

    File::create(source_file).unwrap();

    let result = CopyBuilder::new(source_file, dest_dir).run();

    let dest_created = Path::new(dest_dir).exists();
    std::fs::remove_file(source_file).unwrap();
    let _ = std::fs::remove_dir_all(dest_dir);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidInput);
    assert!(!dest_created, "Destination was created for a file source");
}

#[test]
/// Directories must not be created if they are excluded or don't match an include filter.
fn filters_apply_to_directories() {
    let _ = env_logger::builder().try_init();
    let source_dir = "dir_filter_src";
    let dest_exclude = "dir_filter_exclude_dest";
    let dest_include = "dir_filter_include_dest";

    create_dir_all(format!("{source_dir}/skipme/nested")).unwrap();
    create_dir_all(format!("{source_dir}/sub")).unwrap();
    create_dir_all(format!("{source_dir}/other")).unwrap();
    create_dir_all(format!("{source_dir}/empty")).unwrap();
    create_dir_all(format!("{source_dir}/keep_empty")).unwrap();
    File::create(format!("{source_dir}/a.txt")).unwrap();
    File::create(format!("{source_dir}/sub/b.txt")).unwrap();
    File::create(format!("{source_dir}/other/c.log")).unwrap();
    File::create(format!("{source_dir}/skipme/nested/d.txt")).unwrap();

    CopyBuilder::new(source_dir, dest_exclude)
        .with_exclude_filter("skipme")
        .run()
        .unwrap();
    CopyBuilder::new(source_dir, dest_include)
        .with_include_filter(".txt")
        .with_include_filter("keep")
        .run()
        .unwrap();

    let exists = |p: &str| Path::new(p).exists();
    let excluded_skipme = exists(&format!("{dest_exclude}/skipme"));
    let excluded_sub = exists(&format!("{dest_exclude}/sub/b.txt"));
    let included_a = exists(&format!("{dest_include}/a.txt"));
    let included_b = exists(&format!("{dest_include}/sub/b.txt"));
    let included_d = exists(&format!("{dest_include}/skipme/nested/d.txt"));
    let included_other = exists(&format!("{dest_include}/other"));
    let included_empty = exists(&format!("{dest_include}/empty"));
    let included_keep = exists(&format!("{dest_include}/keep_empty"));

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_exclude).unwrap();
    std::fs::remove_dir_all(dest_include).unwrap();

    assert!(!excluded_skipme, "Excluded directory was created");
    assert!(
        excluded_sub,
        "Directory that is not excluded was not copied"
    );
    assert!(
        included_a && included_b && included_d,
        "Included files in subdirectories were not copied"
    );
    assert!(
        !included_other,
        "Directory without included files was created"
    );
    assert!(
        !included_empty,
        "Empty directory not matching include was created"
    );
    assert!(
        included_keep,
        "Directory matching include filter was not created"
    );
}

#[test]
/// Errors must mention the path they occurred on.
fn errors_contain_path() {
    let _ = env_logger::builder().try_init();
    let source_dir = "error_path_src";
    let dest_dir = "error_path_dest";

    create_dir_all(source_dir).unwrap();
    File::create(format!("{source_dir}/file")).unwrap();
    // A directory in dest where source has a file makes the copy fail
    create_dir_all(format!("{dest_dir}/file")).unwrap();

    let result = CopyBuilder::new(source_dir, dest_dir).overwrite(true).run();

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
    let message = result.unwrap_err().to_string();
    assert!(
        message.contains(&format!("{source_dir}{}file", std::path::MAIN_SEPARATOR)),
        "Error does not contain the path: {}",
        message
    );
}

#[cfg(windows)]
#[test]
/// Symlinks to files and directories must be copied on Windows as well.
fn copy_symlinks_windows() {
    use std::os::windows::fs::{symlink_dir, symlink_file};
    let _ = env_logger::builder().try_init();
    let source_dir = "win_symlink_src";
    let dest_dir = "win_symlink_dest";

    create_dir_all(format!("{source_dir}/dir")).unwrap();
    File::create(format!("{source_dir}/file")).unwrap();
    symlink_file("file", format!("{source_dir}/file_link")).unwrap();
    symlink_dir("dir", format!("{source_dir}/dir_link")).unwrap();

    let result = CopyBuilder::new(source_dir, dest_dir).run();

    let file_link = read_link(format!("{dest_dir}/file_link"));
    let dir_link = read_link(format!("{dest_dir}/dir_link"));
    let dir_link_is_dir = Path::new(&format!("{dest_dir}/dir_link")).is_dir();
    std::fs::remove_dir_all(source_dir).unwrap();
    let _ = std::fs::remove_dir_all(dest_dir);

    result.unwrap();
    assert_eq!(file_link.unwrap(), Path::new("file"));
    assert_eq!(dir_link.unwrap(), Path::new("dir"));
    assert!(
        dir_link_is_dir,
        "Directory symlink was created as file symlink"
    );
}
