use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::Write;
use std::process::{Command, Stdio};

// A long table with moderate-cardinality keys, so the output stays a small
// contingency table while the per-row hot path dominates.
fn make_table(rows: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(rows * 12);
    for r in 0..rows {
        let x = r % 50;
        let y = (r / 50) % 40;
        writeln!(buf, "g{x}\tc{y}\t{}", r % 1000).unwrap();
    }
    buf
}

fn bench_crosstab(c: &mut Criterion) {
    let bin = env!("CARGO_BIN_EXE_rsomics-tsv-crosstab");
    let data = make_table(1_000_000);
    c.bench_function("crosstab count 1e6 rows", |b| {
        b.iter(|| {
            let mut child = Command::new(black_box(bin))
                .args(["--fields", "1,2"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(&data).unwrap();
            assert!(child.wait().unwrap().success());
        });
    });
}

criterion_group!(benches, bench_crosstab);
criterion_main!(benches);
