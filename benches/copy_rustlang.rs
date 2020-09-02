use criterion::*;
use env_logger;
use log::*;
use std::fs::File;
use std::path::Path;
use dircpy::CopyBuilder;

fn bench_rustlang_copy() {
    std::env::set_var("RUST_LOG", "INFO");

    let _ = env_logger::builder()
        // .is_test(true)
        .try_init();

    let url = "https://github.com/rust-lang/rust/archive/master.zip";
    let sample_dir = "rustlang";
    //let output_dir = format!("{}_output", sample_dir);
    let archive = format!("{}.zip", sample_dir);

    info!("Downloading {:?}", url);

    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut out = File::create(&archive).expect("failed to create file");
    std::io::copy(&mut resp, &mut out).expect("failed to copy content");
    info!("Unzipping...");

    let reader = std::fs::File::open(&archive).unwrap();
    unzip::Unzipper::new(reader, sample_dir).unzip().unwrap();
    
    let iterations = 6;
    let mut complete_duration = std::time::Duration::from_nanos(0);
    for i in 0..iterations {
        info!("Iteration {}/{}", i, iterations);
        let start = std::time::Instant::now();
        
        CopyBuilder::new(
            &Path::new(sample_dir).canonicalize().unwrap(),
            &format!("output_{}", i),
        )
        .run()
        .unwrap();
    
        info!("Elapsed time: {:?}", start.elapsed());
        complete_duration += start.elapsed();
    }

    info!("Avg time: {:?}", complete_duration/iterations);
    
    info!("Cleanup");
    std::fs::remove_dir_all(sample_dir).unwrap();
    std::fs::remove_file(archive).unwrap();
    for i in 0..iterations {
        std::fs::remove_dir_all(&format!("output_{}", i)).unwrap();
    }
    info!("Done");
}


fn bench_copy(src: &str, dest: &str) {

        CopyBuilder::new(
            &src,
            &dest,
        )
        .run()
        .unwrap();
    
    }
    





fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

fn cargo_benchmark(c: &mut Criterion) {
    
    let url = "https://github.com/rust-lang/cargo/archive/master.zip";

    // let url = "https://github.com/rust-lang/rust/archive/master.zip";
    let sample_dir = "rustlang";
    //let output_dir = format!("{}_output", sample_dir);
    let archive = format!("{}.zip", sample_dir);

    info!("Downloading {:?}", url);

    let mut resp = reqwest::blocking::get(url).unwrap();
    let mut out = File::create(&archive).expect("failed to create file");
    std::io::copy(&mut resp, &mut out).expect("failed to copy content");
    info!("Unzipping...");

    let reader = std::fs::File::open(&archive).unwrap();
    unzip::Unzipper::new(reader, sample_dir).unzip().unwrap();
    let input = 5u64;
    //c.bench_with_input(BenchmarkId::new("function_name", input), &input |b,i| b.iter(|| bench_copy("rustlang", &format!("output_{}",i))));

    // c.bench_with_input(
    //     BenchmarkId::new("copy rustlang", input), &input,
    //     |b, i| b.iter(|| {
    //         // Code to benchmark using input `i` goes here
    //         bench_copy(sample_dir, &format!("output_{}",i))

    //     }),
    // );

    let mut group = c.benchmark_group("from_elem");
    for size in 0..input {
        
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                bench_copy(sample_dir, &format!("output_{}", size))

            });
        });
    }

    info!("Cleanup");
    std::fs::remove_dir_all(sample_dir).unwrap();
    std::fs::remove_file(archive).unwrap();
    for i in 0..=input {
        std::fs::remove_dir_all(&format!("output_{}", i));
    }
    info!("Done");
}

fn rustlang_benchmark(c: &mut Criterion) {
    
   bench_rustlang_copy();
}

criterion_group!{
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().sample_size(10);
    targets = rustlang_benchmark
}
criterion_main!(benches);