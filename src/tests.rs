use super::*;
use std::fs::create_dir_all;
use std::fs::File;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn copy() {
    create_dir_all("source/level1/level2/level3").unwrap();
    File::create("source/test").unwrap();
    File::create("source/level1/other_file").unwrap();

    #[cfg(unix)]
    {
        File::create("source/exec_file").unwrap();
        std::fs::set_permissions("source/exec_file", std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }

    CopyBuilder::new(
        "source",
        "dest",
    )
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
    }

    // clean up
    std::fs::remove_dir_all("source").unwrap();
    std::fs::remove_dir_all("dest").unwrap();
}

#[test]
fn copy_cargo() {

    let url = "https://github.com/rust-lang/cargo/archive/master.zip";
    let sample_dir = "cargo";
    let output_dir = format!("{}_output", sample_dir);
    let archive = format!("{}.zip", sample_dir);

    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut out = File::create(&archive).expect("failed to create file");
    std::io::copy(&mut resp, &mut out).expect("failed to copy content");

    let reader = std::fs::File::open(&archive).unwrap();

    unzip::Unzipper::new(reader, sample_dir).unzip().unwrap();
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

