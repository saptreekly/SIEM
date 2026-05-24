use criterion::{black_box, criterion_group, criterion_main, Criterion};
use siem::parse_log;

fn bench_parse_log(c: &mut Criterion) {
    let mock_log = "<34>1 2026-05-23T16:00:00Z localhost sshd[1234]: Failed password for root";

    c.bench_function("parse_log_benchmark", |b| {
        b.iter(|| {
            let _ = parse_log(black_box(mock_log));
        })
    });
}

criterion_group!(benches, bench_parse_log);
criterion_main!(benches);
