use super::*;
use std::fs::create_dir_all;
use std::fs::File;
use test::Bencher;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
extern crate test;
use env_logger;


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
    println!("Expanding {}", archive);

    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut out = File::create(&archive).expect("failed to create file");
    std::io::copy(&mut resp, &mut out).expect("failed to copy content");

    let reader = std::fs::File::open(&archive).unwrap();

    unzip::Unzipper::new(reader, sample_dir).unzip().expect("Could not expand cargo sources");
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

#[bench]
fn bench_rustlang_copy(_: &mut Bencher) {
    std::env::set_var("RUST_LOG", "INFO");

    let _ = env_logger::builder()
        // .is_test(true)
        .try_init();

    let url = "https://github.com/rust-lang/rust/archive/master.zip";
    let sample_dir = "sample";
    //let output_dir = format!("{}_output", sample_dir);
    let archive = format!("{}.zip", sample_dir);

    info!("Downloading {:?}", url);

    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut out = File::create(&archive).expect("failed to create file");
    std::io::copy(&mut resp, &mut out).expect("failed to copy content");
    info!("Unzipping...");

    let reader = std::fs::File::open(&archive).unwrap();
    unzip::Unzipper::new(reader, sample_dir).unzip().unwrap();
    
    let iterations = 5;

    for i in 0..iterations {
        info!("Copy bench {}", i);
        let start = std::time::Instant::now();
        
        CopyBuilder::new(
            &Path::new(sample_dir).canonicalize().unwrap(),
            &format!("output_{}", i),
        )
        .run()
        .unwrap();
    
        info!("Elapsed time: {:?}", start.elapsed());
    }
    
    info!("Cleanup");
    std::fs::remove_dir_all(sample_dir).unwrap();
    std::fs::remove_file(archive).unwrap();
    for i in 0..iterations {
        std::fs::remove_dir_all(&format!("output_{}", i)).unwrap();
    }
    info!("Done");

}