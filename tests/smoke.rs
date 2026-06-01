use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rsomics-tsv-crosstab"))
}

fn run(args: &[&str], input: &[u8]) -> std::process::Output {
    let mut child = bin()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn count_contingency_table() {
    let out = run(&["--fields", "1,2"], b"a\tx\na\ty\nb\tx\nb\tx\nc\tz\n");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "\tx\ty\tz\na\t1\t1\tN/A\nb\t2\tN/A\tN/A\nc\tN/A\tN/A\t1\n"
    );
}

#[test]
fn sum_aggregate() {
    let out = run(
        &["--fields", "1,2", "--op", "sum", "--value", "3"],
        b"a\tx\t10\na\ty\t5\nb\tx\t3\nb\tx\t7\n",
    );
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "\tx\ty\na\t10\t5\nb\t10\tN/A\n"
    );
}

#[test]
fn labels_sort_lexicographically_not_numerically() {
    let out = run(&["--fields", "1,2"], b"10\ta\n2\ta\n1\ta\n");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "\ta\n1\t1\n10\t1\n2\t1\n"
    );
}

#[test]
fn default_groups_consecutive_runs_only() {
    // Non-adjacent reoccurrence of (a,x): first run kept, later run discarded.
    let out = run(&["--fields", "1,2"], b"a\tx\nb\tx\na\tx\n");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "\tx\na\t1\nb\t1\n");
}

#[test]
fn sort_aggregates_whole_input() {
    let out = run(&["--fields", "1,2", "--sort"], b"a\tx\nb\tx\na\tx\n");
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "\tx\na\t2\nb\t1\n");
}

#[test]
fn custom_filler() {
    let out = run(&["--fields", "1,2", "--filler", "."], b"a\tx\nb\ty\n");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "\tx\ty\na\t1\t.\nb\t.\t1\n"
    );
}

#[test]
fn empty_input_emits_bare_header() {
    let out = run(&["--fields", "1,2"], b"");
    assert!(out.status.success());
    assert_eq!(out.stdout, b"\n");
}

#[test]
fn short_row_errors() {
    let out = run(&["--fields", "1,2"], b"a\tx\nb\n");
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn bad_op_errors() {
    let out = run(
        &["--fields", "1,2", "--op", "wat", "--value", "3"],
        b"a\tx\t1\n",
    );
    assert!(!out.status.success());
}
