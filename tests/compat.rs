use std::io::Write;
use std::process::{Command, Stdio};

fn ours() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rsomics-tsv-crosstab"))
}

fn datamash_bin() -> Option<String> {
    for cand in [
        "datamash",
        concat!(env!("HOME"), "/miniconda3/bin/datamash"),
    ] {
        let ok = Command::new(cand)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            return Some(cand.to_string());
        }
    }
    None
}

fn pipe(cmd: &mut Command, input: &[u8]) -> std::process::Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

struct Case {
    ours: &'static [&'static str],
    dm: &'static [&'static str],
    input: &'static [u8],
}

const CASES: &[Case] = &[
    // count mode, empty cells filled with N/A
    Case {
        ours: &["--fields", "1,2"],
        dm: &["crosstab", "1,2"],
        input: b"a\tx\na\ty\nb\tx\nb\tx\nc\tz\n",
    },
    // lexicographic (not numeric) label ordering: 1,10,2
    Case {
        ours: &["--fields", "1,2"],
        dm: &["crosstab", "1,2"],
        input: b"10\tb\n2\tb\n1\ta\n10\ta\n",
    },
    // sum aggregate over field 3
    Case {
        ours: &["--fields", "1,2", "--op", "sum", "--value", "3"],
        dm: &["crosstab", "1,2", "sum", "3"],
        input: b"a\tx\t10\na\ty\t5\nb\tx\t3\nb\tx\t7\nc\tz\t1\n",
    },
    // mean → %.14g formatting (5/3 = 1.6666666666667)
    Case {
        ours: &["--fields", "1,2", "--op", "mean", "--value", "3"],
        dm: &["crosstab", "1,2", "mean", "3"],
        input: b"a\tx\t1\na\tx\t2\na\tx\t2\n",
    },
    // min / max / median / sstdev
    Case {
        ours: &["--fields", "1,2", "--op", "min", "--value", "3"],
        dm: &["crosstab", "1,2", "min", "3"],
        input: b"a\tx\t5\na\tx\t2\na\tx\t9\n",
    },
    Case {
        ours: &["--fields", "1,2", "--op", "median", "--value", "3"],
        dm: &["crosstab", "1,2", "median", "3"],
        input: b"a\tx\t1\na\tx\t2\na\tx\t3\na\tx\t4\n",
    },
    Case {
        ours: &["--fields", "1,2", "--op", "sstdev", "--value", "3"],
        dm: &["crosstab", "1,2", "sstdev", "3"],
        input: b"a\tx\t1\na\tx\t2\na\tx\t3\n",
    },
    // textual ops
    Case {
        ours: &["--fields", "1,2", "--op", "unique", "--value", "3"],
        dm: &["crosstab", "1,2", "unique", "3"],
        input: b"a\tx\tq\na\tx\tp\na\tx\tp\n",
    },
    Case {
        ours: &["--fields", "1,2", "--op", "collapse", "--value", "3"],
        dm: &["crosstab", "1,2", "collapse", "3"],
        input: b"a\tx\tq\na\tx\tp\na\tx\tp\n",
    },
    Case {
        ours: &["--fields", "1,2", "--op", "countunique", "--value", "3"],
        dm: &["crosstab", "1,2", "countunique", "3"],
        input: b"a\tx\tq\na\tx\tp\na\tx\tp\n",
    },
    Case {
        ours: &["--fields", "1,2", "--op", "first", "--value", "3"],
        dm: &["crosstab", "1,2", "first", "3"],
        input: b"a\tx\t7\na\tx\t8\n",
    },
    // custom separator
    Case {
        ours: &["--fields", "1,2", "--field-separator", ","],
        dm: &["-t", ",", "crosstab", "1,2"],
        input: b"a,x\na,y\nb,x\n",
    },
    // custom filler
    Case {
        ours: &["--fields", "1,2", "--filler", "."],
        dm: &["--filler=.", "crosstab", "1,2"],
        input: b"a\tx\nb\ty\n",
    },
    // header-in
    Case {
        ours: &["--fields", "1,2", "--header-in"],
        dm: &["--header-in", "crosstab", "1,2"],
        input: b"X\tY\na\tx\na\ty\nb\tx\n",
    },
    // empty-string field values sort first
    Case {
        ours: &["--fields", "1,2"],
        dm: &["crosstab", "1,2"],
        input: b"a\tx\n\tx\na\t\n",
    },
    // single row
    Case {
        ours: &["--fields", "1,2"],
        dm: &["crosstab", "1,2"],
        input: b"a\tx\n",
    },
    // empty input → empty output
    Case {
        ours: &["--fields", "1,2"],
        dm: &["crosstab", "1,2"],
        input: b"",
    },
    // golden fixture
    Case {
        ours: &["--fields", "1,2"],
        dm: &["crosstab", "1,2"],
        input: include_bytes!("golden/data.tsv"),
    },
    Case {
        ours: &["--fields", "1,2", "--op", "sum", "--value", "3"],
        dm: &["crosstab", "1,2", "sum", "3"],
        input: include_bytes!("golden/data.tsv"),
    },
    // Default mode groups only CONSECUTIVE runs and keeps the first run on a
    // later collision — datamash's "input must be pre-sorted" behaviour.
    Case {
        ours: &["--fields", "1,2"],
        dm: &["crosstab", "1,2"],
        input: b"a\tx\na\tx\nb\tx\na\tx\n",
    },
    Case {
        ours: &["--fields", "1,2", "--op", "sum", "--value", "3"],
        dm: &["crosstab", "1,2", "sum", "3"],
        input: b"a\tx\t10\nb\tx\t5\na\tx\t20\n",
    },
    // --sort (-s) sorts first → every key one run → full aggregation.
    Case {
        ours: &["--fields", "1,2", "--sort"],
        dm: &["-s", "crosstab", "1,2"],
        input: b"a\tx\nb\tx\na\tx\n",
    },
    Case {
        ours: &["--fields", "1,2", "--sort", "--op", "sum", "--value", "3"],
        dm: &["-s", "crosstab", "1,2", "sum", "3"],
        input: b"b\ty\t1\na\tx\t2\nb\ty\t3\na\tx\t4\n",
    },
    Case {
        ours: &["--fields", "1,2", "-s"],
        dm: &["-s", "crosstab", "1,2"],
        input: b"10\ta\n2\ta\n1\ta\n10\tb\n",
    },
];

#[test]
fn byte_identical_to_datamash() {
    let Some(dm) = datamash_bin() else {
        eprintln!("skipping: datamash not found (PATH or ~/miniconda3/bin)");
        return;
    };
    for (i, c) in CASES.iter().enumerate() {
        let a = pipe(ours().args(c.ours), c.input);
        let b = pipe(Command::new(&dm).args(c.dm), c.input);
        assert!(
            a.status.success(),
            "case {i}: ours failed on {:?}: {}",
            c.input,
            String::from_utf8_lossy(&a.stderr)
        );
        assert_eq!(
            a.stdout,
            b.stdout,
            "case {i}: stdout mismatch\ninput: {:?}\nours: {:?}\ndm:   {:?}",
            String::from_utf8_lossy(c.input),
            String::from_utf8_lossy(&a.stdout),
            String::from_utf8_lossy(&b.stdout)
        );
    }
}

#[test]
fn missing_field_errors() {
    let Some(dm) = datamash_bin() else {
        eprintln!("skipping: datamash not found");
        return;
    };
    let ragged = b"a\tx\nb\n";
    let a = pipe(ours().args(["--fields", "1,2"]), ragged);
    let b = pipe(Command::new(&dm).args(["crosstab", "1,2"]), ragged);
    assert!(!a.status.success(), "ours should reject short rows");
    assert!(!b.status.success(), "datamash should reject short rows");
    assert!(a.stdout.is_empty());
}
