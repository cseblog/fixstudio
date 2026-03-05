use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Convert a pipe-delimited FIX string to SOH-delimited (raw protocol bytes).
fn to_soh(input: &str) -> String {
    input.chars().map(|c| if c == '|' { '\x01' } else { c }).collect()
}

// ── 100k message file benchmarks ─────────────────────────────────────────────

fn bench_100k(c: &mut Criterion) {
    let pipe_str = fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/sample_100k.fix"),
    )
    .expect("sample_100k.fix not found — run the generator first");

    let soh_str = to_soh(&pipe_str);

    let pipe_bytes = pipe_str.len() as u64;
    let soh_bytes  = soh_str.len()  as u64;

    let mut group = c.benchmark_group("100k_messages");
    group.sample_size(10);

    // ── scalar (current) path, pipe-delimited ──────────────────────────────
    group.throughput(Throughput::Bytes(pipe_bytes));
    group.bench_with_input(
        BenchmarkId::new("scalar_pipe", format!("{:.1} MB", pipe_bytes as f64 / 1_048_576.0)),
        &pipe_str,
        |b, input| b.iter(|| aifixparser::parser::parse_all(black_box(input))),
    );

    // ── AVX2 path, pipe-delimited ─────────────────────────────────────────
    group.throughput(Throughput::Bytes(pipe_bytes));
    group.bench_with_input(
        BenchmarkId::new("avx2_pipe", format!("{:.1} MB", pipe_bytes as f64 / 1_048_576.0)),
        &pipe_str,
        |b, input| b.iter(|| aifixparser::parser::parse_all_simd(black_box(input))),
    );

    // ── scalar (current) path, SOH-delimited ──────────────────────────────
    // The scalar path must allocate & copy to normalise each SOH message.
    group.throughput(Throughput::Bytes(soh_bytes));
    group.bench_with_input(
        BenchmarkId::new("scalar_soh", format!("{:.1} MB", soh_bytes as f64 / 1_048_576.0)),
        &soh_str,
        |b, input| b.iter(|| aifixparser::parser::parse_all(black_box(input))),
    );

    // ── AVX2 path, SOH-delimited ──────────────────────────────────────────
    // The SIMD path skips normalisation entirely — no allocation, one scan.
    group.throughput(Throughput::Bytes(soh_bytes));
    group.bench_with_input(
        BenchmarkId::new("avx2_soh", format!("{:.1} MB", soh_bytes as f64 / 1_048_576.0)),
        &soh_str,
        |b, input| b.iter(|| aifixparser::parser::parse_all_simd(black_box(input))),
    );

    // ── AVX2 bytes path, SOH-delimited ────────────────────────────────────
    // parse_all_simd_bytes: inlined AVX2 scan (no Vec<usize> per message),
    // accepts &[u8] directly (mmap-friendly, no &str conversion).
    let soh_bytes_raw = soh_str.as_bytes();
    group.throughput(Throughput::Bytes(soh_bytes));
    group.bench_with_input(
        BenchmarkId::new("avx2_bytes_soh", format!("{:.1} MB", soh_bytes as f64 / 1_048_576.0)),
        &soh_bytes_raw,
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

    let soh_msg = to_soh(pipe_msg);

    let mut group = c.benchmark_group("single_execution_report");

    group.bench_function("scalar_pipe", |b| {
        b.iter(|| aifixparser::parser::parse_all(black_box(pipe_msg)))
    });

    group.bench_function("avx2_pipe", |b| {
        b.iter(|| aifixparser::parser::parse_all_simd(black_box(pipe_msg)))
    });

    group.bench_function("scalar_soh", |b| {
        b.iter(|| aifixparser::parser::parse_all(black_box(soh_msg.as_str())))
    });

    group.bench_function("avx2_soh", |b| {
        b.iter(|| aifixparser::parser::parse_all_simd(black_box(soh_msg.as_str())))
    });

    group.bench_function("avx2_bytes_soh", |b| {
        b.iter(|| aifixparser::parser::parse_all_simd_bytes(black_box(soh_msg.as_bytes())))
    });

    group.finish();
}

// ── AVX2 scanner microbenchmark ───────────────────────────────────────────────

fn bench_1m_soh(c: &mut Criterion) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/sample_1m_soh.fix");
    let Ok(soh_str) = std::fs::read_to_string(path) else {
        eprintln!("sample_1m_soh.fix not found — run gen_soh_1m.py first");
        return;
    };
    let bytes = soh_str.len() as u64;

    let mut group = c.benchmark_group("1m_messages_soh");
    group.sample_size(10);

    group.throughput(Throughput::Bytes(bytes));
    group.bench_with_input(
        BenchmarkId::new("scalar_soh", format!("{:.0} MB", bytes as f64 / 1_048_576.0)),
        &soh_str,
        |b, input| b.iter(|| aifixparser::parser::parse_all(black_box(input))),
    );

    group.throughput(Throughput::Bytes(bytes));
    group.bench_with_input(
        BenchmarkId::new("avx2_soh", format!("{:.0} MB", bytes as f64 / 1_048_576.0)),
        &soh_str,
        |b, input| b.iter(|| aifixparser::parser::parse_all_simd(black_box(input))),
    );

    let soh_bytes_raw = soh_str.as_bytes();
    group.throughput(Throughput::Bytes(bytes));
    group.bench_with_input(
        BenchmarkId::new("avx2_bytes_soh", format!("{:.0} MB", bytes as f64 / 1_048_576.0)),
        &soh_bytes_raw,
        |b, input| b.iter(|| aifixparser::parser::parse_all_simd_bytes(black_box(input))),
    );

    group.finish();
}

fn bench_scanner(c: &mut Criterion) {
    let pipe_str = fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/sample_100k.fix"),
    )
    .expect("sample_100k.fix not found");
    let soh_str = to_soh(&pipe_str);

    let mut group = c.benchmark_group("delimiter_scanner");
    group.sample_size(10);

    group.throughput(Throughput::Bytes(pipe_str.len() as u64));
    group.bench_function("avx2_find_delimiters_pipe", |b| {
        b.iter(|| aifixparser::simd::find_delimiters(black_box(pipe_str.as_bytes())))
    });

    group.throughput(Throughput::Bytes(soh_str.len() as u64));
    group.bench_function("avx2_find_delimiters_soh", |b| {
        b.iter(|| aifixparser::simd::find_delimiters(black_box(soh_str.as_bytes())))
    });

    group.finish();
}

criterion_group!(benches, bench_100k, bench_1m_soh, bench_single, bench_scanner);
criterion_main!(benches);
