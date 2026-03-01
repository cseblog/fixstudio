use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;

fn bench_parse_100k(c: &mut Criterion) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/sample_100k.fix");
    let input = fs::read_to_string(path).expect("sample_100k.fix not found — run the generator first");
    let bytes = input.len() as u64;

    let mut group = c.benchmark_group("parse_all");
    group.throughput(Throughput::Bytes(bytes));
    group.sample_size(10); // each run ~100-500 ms on 100k messages

    group.bench_with_input(
        BenchmarkId::new("100k_messages", format!("{:.1} MB", bytes as f64 / 1_048_576.0)),
        &input,
        |b, input| b.iter(|| aifixparser::parser::parse_all(black_box(input))),
    );

    group.finish();
}

fn bench_parse_single(c: &mut Criterion) {
    // A realistic 20-field ExecutionReport
    let msg = "8=FIX.4.4|9=153|35=8|34=4|49=EXEC|52=20240115-09:30:01.000|\
               56=BANZAI|6=420.50|11=2000000|14=100|17=1000001|20=0|31=420.50|\
               32=100|37=1000001|38=100|39=2|54=1|55=MSFT|150=F|151=0|10=059|";

    c.bench_function("single_execution_report", |b| {
        b.iter(|| aifixparser::parser::parse_all(black_box(msg)))
    });
}

criterion_group!(benches, bench_parse_100k, bench_parse_single);
criterion_main!(benches);
