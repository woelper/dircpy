# Benchmarks run and run_par against cp -r, and lms if installed (cargo install lms)
cargo bench
echo "Cleanup"
rm -rf bench_data
