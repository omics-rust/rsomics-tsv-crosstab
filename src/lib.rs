use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

use rsomics_common::{Result, RsomicsError};

mod fmtg;
mod ops;

pub use ops::Op;

pub struct Config {
    pub delim: u8,
    /// 1-based field indices selecting the x (row) and y (column) keys.
    pub x: usize,
    pub y: usize,
    /// Aggregate to apply, with the 1-based field it reads. `None` = count co-occurrences.
    pub agg: Option<(Op, usize)>,
    /// Sort input by the key fields first. Without it, like datamash, only
    /// consecutive runs of a key are grouped (input is assumed pre-sorted).
    pub sort: bool,
    /// First input line is column headers and is dropped (datamash --header-in).
    pub header_in: bool,
    pub filler: Vec<u8>,
    pub collapse_delim: u8,
}

pub fn run(input: &Path, cfg: &Config, out: &mut dyn Write) -> Result<()> {
    let mut raw = Vec::new();
    open(input)?.read_to_end(&mut raw)?;
    let mut bw = BufWriter::with_capacity(1 << 20, out);
    crosstab(&raw, cfg, &mut bw)?;
    bw.flush().map_err(RsomicsError::Io)
}

fn open(input: &Path) -> Result<Box<dyn Read>> {
    if input.as_os_str() == "-" {
        Ok(Box::new(std::io::stdin().lock()))
    } else {
        Ok(Box::new(File::open(input).map_err(|e| {
            RsomicsError::InvalidInput(format!("{}: {e}", input.display()))
        })?))
    }
}

// datamash groups only *consecutive* runs of a key (the input is assumed
// pre-sorted); when a cell's key reappears in a later run the first run's value
// is kept and the rest discarded. `--sort` reorders the rows by key first, which
// makes every key one contiguous run and so aggregates the whole input. Cells
// are keyed by interned (x,y) indices so the run loop touches only integers.
fn crosstab(raw: &[u8], cfg: &Config, out: &mut dyn Write) -> Result<()> {
    // datamash's getline drops a single trailing newline; empty input still
    // emits the (empty) header line.
    let body = raw.strip_suffix(b"\n").unwrap_or(raw);
    if body.is_empty() {
        return out.write_all(b"\n").map_err(RsomicsError::Io);
    }

    let need = cfg.x.max(cfg.y).max(cfg.agg.map_or(0, |(_, f)| f));

    let mut state = RunState {
        cfg,
        need,
        xs: BTreeMap::new(),
        ys: BTreeMap::new(),
        cells: BTreeMap::new(),
        field_buf: Vec::with_capacity(need + 1),
        run: None,
    };

    let skip = usize::from(cfg.header_in);
    if cfg.sort {
        let mut lines: Vec<&[u8]> = body.split(|&b| b == b'\n').skip(skip).collect();
        lines.sort_by_key(|line| key(line, cfg));
        for (lineno, line) in lines.into_iter().enumerate() {
            state.feed(lineno + skip, line)?;
        }
    } else {
        for (lineno, line) in body.split(|&b| b == b'\n').enumerate().skip(skip) {
            state.feed(lineno, line)?;
        }
    }
    state.finish();

    emit(cfg, &state.xs, &state.ys, &state.cells, out)
}

struct RunState<'a> {
    cfg: &'a Config,
    need: usize,
    xs: BTreeMap<Vec<u8>, usize>,
    ys: BTreeMap<Vec<u8>, usize>,
    cells: BTreeMap<(usize, usize), ops::Acc>,
    field_buf: Vec<(usize, usize)>,
    run: Option<Run>,
}

// The open run holds the current cell's interned key, the label bytes (so a
// row continuing the run is recognised by a byte compare, with no map lookup or
// allocation), and the accumulator.
struct Run {
    xi: usize,
    yi: usize,
    xlabel: Vec<u8>,
    ylabel: Vec<u8>,
    acc: ops::Acc,
}

impl RunState<'_> {
    fn feed(&mut self, lineno: usize, line: &[u8]) -> Result<()> {
        split_fields(line, self.cfg.delim, &mut self.field_buf);
        if self.field_buf.len() < self.need {
            return Err(RsomicsError::InvalidInput(format!(
                "invalid input: field {} requested, line {} has only {} fields",
                self.need,
                lineno + 1,
                self.field_buf.len(),
            )));
        }
        let xv = field(line, &self.field_buf, self.cfg.x);
        let yv = field(line, &self.field_buf, self.cfg.y);

        let continues = matches!(&self.run, Some(r) if r.xlabel == xv && r.ylabel == yv);
        if !continues {
            if let Some(r) = self.run.take() {
                self.cells.entry((r.xi, r.yi)).or_insert(r.acc);
            }
            let xi = intern(&mut self.xs, xv);
            let yi = intern(&mut self.ys, yv);
            self.run = Some(Run {
                xi,
                yi,
                xlabel: xv.to_vec(),
                ylabel: yv.to_vec(),
                acc: ops::Acc::new(self.cfg.agg.map(|(op, _)| op)),
            });
        }
        let acc = &mut self.run.as_mut().unwrap().acc;
        match self.cfg.agg {
            None => acc.bump(),
            Some((_, f)) => acc.push(field(line, &self.field_buf, f), lineno + 1)?,
        }
        Ok(())
    }

    fn finish(&mut self) {
        if let Some(r) = self.run.take() {
            self.cells.entry((r.xi, r.yi)).or_insert(r.acc);
        }
    }
}

fn key(line: &[u8], cfg: &Config) -> (Vec<u8>, Vec<u8>) {
    let mut buf = Vec::new();
    split_fields(line, cfg.delim, &mut buf);
    let g = |idx1: usize| -> Vec<u8> {
        buf.get(idx1 - 1)
            .map(|&(s, e)| line[s..e].to_vec())
            .unwrap_or_default()
    };
    (g(cfg.x), g(cfg.y))
}

// An empty line is zero fields, matching datamash's getline-based parser; a
// non-empty line has (delimiter count + 1) fields.
fn split_fields(line: &[u8], delim: u8, out: &mut Vec<(usize, usize)>) {
    out.clear();
    if line.is_empty() {
        return;
    }
    let mut start = 0;
    for (i, &b) in line.iter().enumerate() {
        if b == delim {
            out.push((start, i));
            start = i + 1;
        }
    }
    out.push((start, line.len()));
}

fn field<'a>(line: &'a [u8], spans: &[(usize, usize)], idx1: usize) -> &'a [u8] {
    let (s, e) = spans[idx1 - 1];
    &line[s..e]
}

fn intern(table: &mut BTreeMap<Vec<u8>, usize>, key: &[u8]) -> usize {
    let next = table.len();
    *table.entry(key.to_vec()).or_insert(next)
}

fn emit(
    cfg: &Config,
    xs: &BTreeMap<Vec<u8>, usize>,
    ys: &BTreeMap<Vec<u8>, usize>,
    cells: &BTreeMap<(usize, usize), ops::Acc>,
    out: &mut dyn Write,
) -> Result<()> {
    // BTreeMap iterates keys in sorted byte order; map each label to its row /
    // column slot index in that order.
    let x_slot: Vec<(usize, &[u8])> = xs.iter().map(|(k, &i)| (i, k.as_slice())).collect();
    let y_slot: Vec<(usize, &[u8])> = ys.iter().map(|(k, &i)| (i, k.as_slice())).collect();

    let mut line = Vec::with_capacity(256);
    for (_, label) in &y_slot {
        line.push(cfg.delim);
        line.extend_from_slice(label);
    }
    line.push(b'\n');
    out.write_all(&line).map_err(RsomicsError::Io)?;

    for (xi, xlabel) in &x_slot {
        line.clear();
        line.extend_from_slice(xlabel);
        for (yi, _) in &y_slot {
            line.push(cfg.delim);
            match cells.get(&(*xi, *yi)) {
                Some(acc) => acc.render(cfg.collapse_delim, &mut line),
                None => line.extend_from_slice(&cfg.filler),
            }
        }
        line.push(b'\n');
        out.write_all(&line).map_err(RsomicsError::Io)?;
    }
    Ok(())
}
