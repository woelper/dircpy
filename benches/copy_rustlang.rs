use criterion::*;
use env_logger;
use log::*;
use std::fs::File;
use std::path::Path;
use dircpy::CopyBuilder;





fn download_and_unpack(url: &str, name: &str) {
    // let url = "";
    let sample_dir = name;
    let archive = format!("{}.zip", sample_dir);

    if !std::path::Path::new(&archive).is_file() {
        info!("Downloading {:?}", url);
    
        let mut resp = reqwest::blocking::get(url).unwrap();
        let mut out = File::create(&archive).expect("failed to create file");
        std::io::copy(&mut resp, &mut out).expect("failed to copy content");
        
    }

    info!("Unzipping...");
    let reader = std::fs::File::open(&archive).unwrap();
    unzip::Unzipper::new(reader, sample_dir).unzip().unwrap();
    info!("Done. Ready.");

}


fn test_cp(c: &mut Criterion) {
    std::env::set_var("RUST_LOG", "INFO");
    let _ = env_logger::builder()
        .try_init();
    // One-time setup code goes here
    let source = "rustlang";
    let dest = "test";
    download_and_unpack("https://github.com/rust-lang/rust/archive/master.zip", source);
    c.bench_function("cp -r", |b| {
        // Per-sample (note that a sample can be many iterations) setup goes here
        b.iter(|| {
            // Measured code goes here
            std::process::Command::new("cp").arg("-r").arg(source).arg(dest).output();

        });
    });

    std::fs::remove_dir_all(source).unwrap();
    std::fs::remove_dir_all(dest).unwrap();
}


fn test_cpy(c: &mut Criterion) {
    std::env::set_var("RUST_LOG", "INFO");
    let _ = env_logger::builder()
        .try_init();
    // One-time setup code goes here
    let source = "rustlang";
    let dest = "test";
    download_and_unpack("https://github.com/rust-lang/rust/archive/master.zip", source);
    c.bench_function("cpy", |b| {
        // Per-sample (note that a sample can be many iterations) setup goes here
        b.iter(|| {
            // Measured code goes here
            CopyBuilder::new(
                &source,
                &dest,
            )
            .overwrite(true)
            .run()
            .unwrap();
        });
    });

    std::fs::remove_dir_all(source).unwrap();
    std::fs::remove_dir_all(dest).unwrap();
}


fn test_lms(c: &mut Criterion) {
    std::env::set_var("RUST_LOG", "INFO");
    let _ = env_logger::builder()
        .try_init();
    // One-time setup code goes here
    let source = "rustlang";
    let dest = "test";
    download_and_unpack("https://github.com/rust-lang/rust/archive/master.zip", source);
    c.bench_function("lms", |b| {
        // Per-sample (note that a sample can be many iterations) setup goes here
        b.iter(|| {
            // Measured code goes here
            std::process::Command::new("lms").arg("cp").arg(source).arg(dest).output();

        });
    });

    std::fs::remove_dir_all(source).unwrap();
    std::fs::remove_dir_all(dest).unwrap();
}

criterion_group!{
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default()
    .sample_size(10)
    .warm_up_time(std::time::Duration::from_secs(3))
    .measurement_time(std::time::Duration::from_secs(30))
    ;
    targets = test_cpy, test_cp, test_lms
}
criterion_main!(benches);