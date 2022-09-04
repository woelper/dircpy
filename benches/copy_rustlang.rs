use criterion::*;
use dircpy::CopyBuilder;
use env_logger;
use log::*;
use std::fs::File;
use unzip::Unzipper;

//const SAMPLE_DATA:  &str = "https://github.com/rust-lang/rust/archive/master.zip";
const SAMPLE_DATA: &str = "https://github.com/rust-lang/cargo/archive/master.zip";
const SOURCE: &str = "bench_data/source";
const DEST: &str = "bench_data/dest";

fn random_string() -> String {
    format!("{:?}", std::time::Instant::now())
}

fn download_and_unpack(url: &str, name: &str) {
    let archive = format!("{}.zip", name);

    if !std::path::Path::new(&archive).is_file() {
        info!("Downloading {:?}", url);

        let mut resp = reqwest::blocking::get(url).unwrap();
        let mut out = File::create(&archive).expect("failed to create file");
        std::io::copy(&mut resp, &mut out).expect("failed to copy content");
    } else {
        info!("Did not download, archive already present");
    }

    info!("Unzipping...");

    Unzipper::new(File::open(&archive).unwrap(), name)
        .unzip()
        .unwrap();
    info!("Done. Ready.");
}

fn setup(_: &mut Criterion) {
    std::env::set_var("RUST_LOG", "INFO");
    let _ = env_logger::builder().try_init();
    std::fs::create_dir_all(SOURCE).unwrap();
    download_and_unpack(SAMPLE_DATA, SOURCE);
}

fn teardown(_: &mut Criterion) {
    std::env::set_var("RUST_LOG", "INFO");
    let _ = env_logger::builder().try_init();
    // One-time setup code goes here
    info!("CLEANUP");
    // let _ = std::fs::remove_dir_all(source);
    let _ = std::fs::remove_dir_all(DEST);
    // let _ = std::fs::remove_file(archive);
    info!("DONE");
}

fn test_cp(c: &mut Criterion) {
    std::env::set_var("RUST_LOG", "INFO");
    let _ = env_logger::builder().try_init();
    // One-time setup code goes here
    c.bench_function("cp -r", |b| {
        // Per-sample (note that a sample can be many iterations) setup goes here
        b.iter(|| {
            // Measured code goes here
            std::process::Command::new("cp")
                .arg("-r")
                .arg(SOURCE)
                .arg(&format!("{}{}", DEST, random_string()))
                .output().unwrap();
        });
    });
}


fn test_dircpy_single(c: &mut Criterion) {
    // One-time setup code goes here
    // download_and_unpack(SAMPLE_DATA, source);
    c.bench_function("cpy single threaded", |b| {
        // Per-sample (note that a sample can be many iterations) setup goes here
        b.iter(|| {
            // Measured code goes here
            CopyBuilder::new(&SOURCE, &format!("{}{}", DEST, random_string()))
                .overwrite(true)
                .run()
                .unwrap();
        });
    });
}

fn test_dircpy_parallel(c: &mut Criterion) {
    // One-time setup code goes here
    #[cfg(feature = "jwalk")]
    c.bench_function("cpy multi-threaded", |b| {
        // Per-sample (note that a sample can be many iterations) setup goes here
        b.iter(|| {
            // Measured code goes here
            CopyBuilder::new(&SOURCE, &format!("{}{}", DEST, random_string()))
                .overwrite(true)
                .run_par()
                .unwrap();
        });
    });
}

fn test_lms(c: &mut Criterion) {
    std::env::set_var("RUST_LOG", "INFO");
    let _ = env_logger::builder().try_init();
    // One-time setup code goes here
    // download_and_unpack(SAMPLE_DATA, source);
    c.bench_function("lms", |b| {
        // Per-sample (note that a sample can be many iterations) setup goes here
        b.iter(|| {
            // Measured code goes here
            std::process::Command::new("lms")
                .arg("cp")
                .arg(SOURCE)
                .arg(&format!("{}{}", DEST, random_string()))
                .output().unwrap();
        });
    });
}

criterion_group! {
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default()
    .sample_size(10)
    // .sampling_mode()
    .warm_up_time(std::time::Duration::from_secs(4))
    .measurement_time(std::time::Duration::from_secs(6))
    ;
    targets = setup, test_dircpy_single, test_dircpy_parallel, test_cp, test_lms, teardown
}
criterion_main!(benches);
