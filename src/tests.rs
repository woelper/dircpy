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

#[cfg(feature = "jwalk")]
#[test]
fn copy_subdir_jwalk() {
    use std::fs::File;
    use std::io::Write;

    let source_dir = "overwrite_sourcej";
    let dest_dir = "overwrite_destj";

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

// Bug 2: when both overwrite_if_newer and overwrite_if_size_differs are set,
// the copy should happen if EITHER condition is true (OR semantics), not both (AND).

#[test]
fn overwrite_or_newer_same_size() {
    // Source IS newer, same size → should copy even though size is unchanged.
    use std::fs::File;
    use std::io::Write;

    let source_dir = "or_newer_same_size_src";
    let dest_dir = "or_newer_same_size_dst";

    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();
    create_dir_all(source_dir).unwrap();
    create_dir_all(dest_dir).unwrap();

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

    CopyBuilder::new(source_dir, dest_dir)
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

#[test]
fn overwrite_or_size_differs_not_newer() {
    // Source is NOT newer, but sizes differ → should copy even though source is older.
    use std::fs::File;
    use std::io::Write;

    let source_dir = "or_size_diff_not_newer_src";
    let dest_dir = "or_size_diff_not_newer_dst";

    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();
    create_dir_all(source_dir).unwrap();
    create_dir_all(dest_dir).unwrap();

    // Write source first so it is older.
    let source_content = "short";
    let mut f = File::create(format!("{source_dir}/file.txt")).unwrap();
    write!(f, "{source_content}").unwrap();
    drop(f);

    // Small delay so the dest mtime is strictly newer.
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Write dest after with different (larger) content, making it newer AND bigger.
    let mut f = File::create(format!("{dest_dir}/file.txt")).unwrap();
    write!(f, "much longer destination content").unwrap();
    drop(f);

    // Source is older but smaller; sizes differ, so should be copied.
    CopyBuilder::new(source_dir, dest_dir)
        .overwrite_if_newer(true)
        .overwrite_if_size_differs(true)
        .run()
        .unwrap();

    let result = std::fs::read_to_string(format!("{dest_dir}/file.txt")).unwrap();
    assert_eq!(
        result, source_content,
        "File should be overwritten because sizes differ (even though source is not newer)"
    );

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
}

// Bug 3: the same OR-semantics bug existed in copy_file() used by run_par().

#[cfg(feature = "jwalk")]
#[test]
fn overwrite_or_newer_same_size_par() {
    // Source IS newer, same size → run_par() should copy.
    use std::fs::File;
    use std::io::Write;

    let source_dir = "or_newer_same_size_par_src";
    let dest_dir = "or_newer_same_size_par_dst";

    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();
    create_dir_all(source_dir).unwrap();
    create_dir_all(dest_dir).unwrap();

    let mut f = File::create(format!("{dest_dir}/file.txt")).unwrap();
    write!(f, "hello123").unwrap();
    drop(f);

    std::thread::sleep(std::time::Duration::from_millis(50));

    let source_content = "world456";
    let mut f = File::create(format!("{source_dir}/file.txt")).unwrap();
    write!(f, "{source_content}").unwrap();
    drop(f);

    CopyBuilder::new(source_dir, dest_dir)
        .overwrite_if_newer(true)
        .overwrite_if_size_differs(true)
        .run_par()
        .unwrap();

    let result = std::fs::read_to_string(format!("{dest_dir}/file.txt")).unwrap();
    assert_eq!(
        result, source_content,
        "run_par: file should be overwritten because source is newer (even though size is the same)"
    );

    std::fs::remove_dir_all(source_dir).unwrap();
    std::fs::remove_dir_all(dest_dir).unwrap();
}

#[cfg(feature = "jwalk")]
#[test]
fn overwrite_or_size_differs_not_newer_par() {
    // Source is NOT newer, but sizes differ → run_par() should copy.
    use std::fs::File;
    use std::io::Write;

    let source_dir = "or_size_diff_not_newer_par_src";
    let dest_dir = "or_size_diff_not_newer_par_dst";

    std::env::set_var("RUST_LOG", "debug");
    let _ = env_logger::try_init();
    create_dir_all(source_dir).unwrap();
    create_dir_all(dest_dir).unwrap();

    let source_content = "short";
    let mut f = File::create(format!("{source_dir}/file.txt")).unwrap();
    write!(f, "{source_content}").unwrap();
    drop(f);

    std::thread::sleep(std::time::Duration::from_millis(50));

    let mut f = File::create(format!("{dest_dir}/file.txt")).unwrap();
    write!(f, "much longer destination content").unwrap();
    drop(f);

    CopyBuilder::new(source_dir, dest_dir)
        .overwrite_if_newer(true)
        .overwrite_if_size_differs(true)
        .run_par()
        .unwrap();

    let result = std::fs::read_to_string(format!("{dest_dir}/file.txt")).unwrap();
    assert_eq!(
        result, source_content,
        "run_par: file should be overwritten because sizes differ (even though source is not newer)"
    );

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
    let num_input_files = WalkDir::new(&sample_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .count();

    CopyBuilder::new(
        &Path::new(sample_dir).canonicalize().unwrap(),
        &PathBuf::from(&output_dir),
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
    let num_input_files = WalkDir::new(&sample_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .count();

    CopyBuilder::new(
        &Path::new(&sample_dir).canonicalize().unwrap(),
        &PathBuf::from(&output_dir),
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
