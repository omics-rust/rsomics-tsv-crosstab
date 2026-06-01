use std::collections::BTreeSet;

use rsomics_common::{Result, RsomicsError};

use crate::fmtg;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Count,
    Sum,
    Min,
    Max,
    Absmin,
    Absmax,
    Range,
    Mean,
    Median,
    Q1,
    Q3,
    Iqr,
    Pstdev,
    Sstdev,
    Pvar,
    Svar,
    First,
    Last,
    Unique,
    Collapse,
    Countunique,
}

impl Op {
    pub fn parse(name: &str) -> Option<Op> {
        Some(match name {
            "count" => Op::Count,
            "sum" => Op::Sum,
            "min" => Op::Min,
            "max" => Op::Max,
            "absmin" => Op::Absmin,
            "absmax" => Op::Absmax,
            "range" => Op::Range,
            "mean" => Op::Mean,
            "median" => Op::Median,
            "q1" => Op::Q1,
            "q3" => Op::Q3,
            "iqr" => Op::Iqr,
            "pstdev" => Op::Pstdev,
            "sstdev" => Op::Sstdev,
            "pvar" => Op::Pvar,
            "svar" => Op::Svar,
            "first" => Op::First,
            "last" => Op::Last,
            "unique" => Op::Unique,
            "collapse" => Op::Collapse,
            "countunique" => Op::Countunique,
            _ => return None,
        })
    }

    fn numeric(self) -> bool {
        !matches!(
            self,
            Op::Count | Op::First | Op::Last | Op::Unique | Op::Collapse | Op::Countunique
        )
    }
}

pub enum Acc {
    Count(u64),
    Nums { op: Op, vals: Vec<f64> },
    Strs { op: Op, vals: Vec<Vec<u8>> },
}

impl Acc {
    pub fn new(op: Option<Op>) -> Acc {
        match op {
            None | Some(Op::Count) => Acc::Count(0),
            Some(op) if op.numeric() => Acc::Nums {
                op,
                vals: Vec::new(),
            },
            Some(op) => Acc::Strs {
                op,
                vals: Vec::new(),
            },
        }
    }

    pub fn bump(&mut self) {
        if let Acc::Count(n) = self {
            *n += 1;
        }
    }

    pub fn push(&mut self, raw: &[u8], lineno: usize) -> Result<()> {
        match self {
            Acc::Count(n) => *n += 1,
            Acc::Nums { vals, .. } => {
                let s = std::str::from_utf8(raw)
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .ok_or_else(|| {
                        RsomicsError::InvalidInput(format!(
                            "invalid numeric value in line {lineno}: '{}'",
                            String::from_utf8_lossy(raw)
                        ))
                    })?;
                vals.push(s);
            }
            Acc::Strs { vals, .. } => vals.push(raw.to_vec()),
        }
        Ok(())
    }

    pub fn render(&self, collapse_delim: u8, out: &mut Vec<u8>) {
        match self {
            Acc::Count(n) => out.extend_from_slice(itoa(*n).as_bytes()),
            Acc::Nums { op, vals } => fmtg::write_g(reduce(*op, vals), out),
            Acc::Strs { op, vals } => render_str(*op, vals, collapse_delim, out),
        }
    }
}

fn itoa(n: u64) -> String {
    n.to_string()
}

fn reduce(op: Op, vals: &[f64]) -> f64 {
    match op {
        Op::Sum => vals.iter().sum(),
        Op::Min => vals.iter().copied().fold(f64::INFINITY, f64::min),
        Op::Max => vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        Op::Absmin => vals.iter().map(|v| v.abs()).fold(f64::INFINITY, f64::min),
        Op::Absmax => vals
            .iter()
            .map(|v| v.abs())
            .fold(f64::NEG_INFINITY, f64::max),
        Op::Range => {
            let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            hi - lo
        }
        Op::Mean => mean(vals),
        Op::Median => percentile(vals, 0.5),
        Op::Q1 => percentile(vals, 0.25),
        Op::Q3 => percentile(vals, 0.75),
        Op::Iqr => percentile(vals, 0.75) - percentile(vals, 0.25),
        Op::Pvar => variance(vals, 0),
        Op::Svar => variance(vals, 1),
        Op::Pstdev => variance(vals, 0).sqrt(),
        Op::Sstdev => variance(vals, 1).sqrt(),
        _ => unreachable!("non-numeric op in reduce"),
    }
}

fn mean(vals: &[f64]) -> f64 {
    vals.iter().sum::<f64>() / vals.len() as f64
}

fn variance(vals: &[f64], ddof: usize) -> f64 {
    let m = mean(vals);
    let ss: f64 = vals.iter().map(|v| (v - m) * (v - m)).sum();
    ss / (vals.len() - ddof) as f64
}

// datamash's percentile: sort, then linear interpolation between the two
// nearest ranks (the same scheme R calls type-7).
fn percentile(vals: &[f64], p: f64) -> f64 {
    let mut v = vals.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n == 1 {
        return v[0];
    }
    let rank = p * (n - 1) as f64;
    let lo = rank.floor() as usize;
    let frac = rank - lo as f64;
    if lo + 1 < n {
        v[lo] + frac * (v[lo + 1] - v[lo])
    } else {
        v[lo]
    }
}

fn render_str(op: Op, vals: &[Vec<u8>], delim: u8, out: &mut Vec<u8>) {
    match op {
        Op::First => out.extend_from_slice(&vals[0]),
        Op::Last => out.extend_from_slice(&vals[vals.len() - 1]),
        Op::Collapse => join(vals.iter().map(Vec::as_slice), delim, out),
        Op::Unique => {
            let set: BTreeSet<&[u8]> = vals.iter().map(Vec::as_slice).collect();
            join(set.into_iter(), delim, out);
        }
        Op::Countunique => {
            let set: BTreeSet<&[u8]> = vals.iter().map(Vec::as_slice).collect();
            out.extend_from_slice(set.len().to_string().as_bytes());
        }
        _ => unreachable!("numeric op in render_str"),
    }
}

fn join<'a>(items: impl Iterator<Item = &'a [u8]>, delim: u8, out: &mut Vec<u8>) {
    let mut first = true;
    for it in items {
        if !first {
            out.push(delim);
        }
        first = false;
        out.extend_from_slice(it);
    }
}
