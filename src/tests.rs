use super::*;
use std::fs::create_dir_all;
use std::fs::File;
#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};

#[test]
fn copy_basic() {
    create_dir_all("source/level1/level2/level3").unwrap();
    File::create("source/test").unwrap();
    File::create("source/level1/other_file").unwrap();

    #[cfg(unix)]
    {
        File::create("source/exec_file").unwrap();
        std::fs::set_permissions("source/exec_file", std::fs::Permissions::from_mode(0o755))
            .unwrap();
        symlink("exec_file", "source/symlink").unwrap();
        symlink("does_not_exist", "source/dangling_symlink").unwrap();
    }

    CopyBuilder::new("source", "dest")
        .overwrite(true)
        .overwrite_if_newer(true)
        .run()
        .unwrap();

    #[cfg(unix)]
    {
        let f = File::open("dest/exec_file").unwrap();
        let metadata = f.metadata().unwrap();
        let permissions = metadata.permissions();
        println!("permissions: {:o}", permissions.mode());
        assert_eq!(permissions.mode(), 33261);
        assert_eq!(
            Path::new("exec_file"),
            read_link("dest/symlink").unwrap().as_path()
        );
        assert_eq!(
            Path::new("does_not_exist"),
            read_link("dest/dangling_symlink").unwrap().as_path()
        );
    }

    // clean up
    std::fs::remove_dir_all("source").unwrap();
    std::fs::remove_dir_all("dest").unwrap();
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

    CopyBuilder::new("source", "source/subdir")
    .run()
    .unwrap();

    // std::fs::remove_dir_all("source").unwrap();
}


#[test]
fn copy_exclude() {
    std::env::set_var("RUST_LOG", "DEBUG");
    let _ = env_logger::builder().try_init();

    let src = "ex_src";
    let dst = "ex_dest";

    create_dir_all(src).unwrap();
    File::create(format!("{}/foo", src)).unwrap();
    File::create(format!("{}/bar", src)).unwrap();

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
    File::create(format!("{}/foo", src)).unwrap();
    File::create(format!("{}/bar", src)).unwrap();
    File::create(format!("{}/baz", src)).unwrap();

    CopyBuilder::new(src, dst)
        .overwrite(true)
        .overwrite_if_newer(true)
        .with_include_filter("foo")
        .with_include_filter("baz")
        .run()
        .unwrap();

    assert!(Path::new(&format!("{}/foo", dst)).is_file());
    assert!(!Path::new(&format!("{}/bar", dst)).exists());
    assert!(Path::new(&format!("{}/baz", dst)).exists());

    // clean up
    std::fs::remove_dir_all(src).unwrap();
    std::fs::remove_dir_all(dst).unwrap();
}

#[test]
fn copy_cargo() {
    let url = "https://github.com/rust-lang/cargo/archive/master.zip";
    let sample_dir = "cargo";
    let output_dir = format!("{}_output", sample_dir);
    let archive = format!("{}.zip", sample_dir);
    println!("Expanding {}", archive);

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
