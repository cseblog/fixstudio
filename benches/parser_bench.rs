use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn to_soh(input: &str) -> Vec<u8> {
    input.bytes().map(|b| if b == b'|' { 0x01 } else { b }).collect()
}

// ── 100k message file benchmarks ─────────────────────────────────────────────

fn bench_100k(c: &mut Criterion) {
    let pipe_str = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/fix_health_test_100k.log"),
    )
    .expect("fixtures/fix_health_test_100k.log not found — run: cargo run --release --bin gen_fix");

    let soh_bytes = to_soh(&pipe_str);
    let mb = |n: usize| format!("{:.1} MB", n as f64 / 1_048_576.0);

    let mut group = c.benchmark_group("100k_messages");
    group.sample_size(10);

    // Baseline: normalize + parallel scalar parse.
    group.throughput(Throughput::Bytes(pipe_str.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("parse_all_pipe", mb(pipe_str.len())),
        &pipe_str,
        |b, input| b.iter(|| aifixparser::parser::parse_all(black_box(input))),
    );

    // Hot path: SIMD bytes, SOH-delimited (what loader.rs uses in production).
    group.throughput(Throughput::Bytes(soh_bytes.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("simd_bytes_soh", mb(soh_bytes.len())),
        &soh_bytes,
        |b, input| b.iter(|| aifixparser::parser::parse_all_simd_bytes(black_box(input))),
    );

    // Hot path: SIMD bytes, pipe-delimited (production path for pipe files).
    group.throughput(Throughput::Bytes(pipe_str.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("simd_bytes_pipe", mb(pipe_str.len())),
        pipe_str.as_bytes(),
        |b, input| b.iter(|| aifixparser::parser::parse_all_simd_bytes(black_box(input))),
    );

    group.finish();
}

// ── 1M message file benchmarks ────────────────────────────────────────────────

fn bench_1m(c: &mut Criterion) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/fix_test_1m.log");
    let Ok(content) = std::fs::read(path) else {
        eprintln!("fixtures/fix_test_1m.log not found — run: cargo run --release --bin gen_fix");
        return;
    };
    let mb = |n: usize| format!("{:.0} MB", n as f64 / 1_048_576.0);
    let bytes = content.len() as u64;

    let mut group = c.benchmark_group("1m_messages");
    group.sample_size(10);

    group.throughput(Throughput::Bytes(bytes));
    group.bench_with_input(
        BenchmarkId::new("simd_bytes", mb(content.len())),
        &content,
        |b, input| b.iter(|| aifixparser::parser::parse_all_simd_bytes(black_box(input))),
    );

    group.finish();
}

// ── Single-message microbenchmarks ───────────────────────────────────────────

fn bench_single(c: &mut Criterion) {
    let pipe_msg =
        "8=FIX.4.4|9=153|35=8|34=4|49=EXEC|52=20240115-09:30:01.000|\
         56=BANZAI|6=420.50|11=2000000|14=100|17=1000001|20=0|31=420.50|\
         32=100|37=1000001|38=100|39=2|54=1|55=MSFT|150=F|151=0|10=059|";

    let soh_bytes = to_soh(pipe_msg);

    let mut group = c.benchmark_group("single_execution_report");

    group.bench_function("parse_all_pipe", |b| {
        b.iter(|| aifixparser::parser::parse_all(black_box(pipe_msg)))
    });

    group.bench_function("simd_bytes_pipe", |b| {
        b.iter(|| aifixparser::parser::parse_all_simd_bytes(black_box(pipe_msg.as_bytes())))
    });

    group.bench_function("simd_bytes_soh", |b| {
        b.iter(|| aifixparser::parser::parse_all_simd_bytes(black_box(soh_bytes.as_slice())))
    });

    group.finish();
}

// ── Delimiter scanner microbenchmark ─────────────────────────────────────────

fn bench_scanner(c: &mut Criterion) {
    let pipe_str = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/fix_health_test_100k.log"),
    )
    .expect("fixtures/fix_health_test_100k.log not found");
    let soh_bytes = to_soh(&pipe_str);

    let mut group = c.benchmark_group("delimiter_scanner");
    group.sample_size(10);

    group.throughput(Throughput::Bytes(pipe_str.len() as u64));
    group.bench_function("find_delimiters_pipe", |b| {
        b.iter(|| aifixparser::simd::find_delimiters(black_box(pipe_str.as_bytes())))
    });

    group.throughput(Throughput::Bytes(soh_bytes.len() as u64));
    group.bench_function("find_delimiters_soh", |b| {
        b.iter(|| aifixparser::simd::find_delimiters(black_box(soh_bytes.as_slice())))
    });

    group.finish();
}

criterion_group!(benches, bench_single, bench_100k, bench_1m, bench_scanner);
criterion_main!(benches);
