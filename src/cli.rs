use std::path::PathBuf;

use clap::Parser;
use rsomics_common::{CommonFlags, Result, RsomicsError, Tool, ToolMeta};
use rsomics_help::{Example, FlagSpec, HelpSpec, Origin, Section};
use rsomics_tsv_crosstab::{Config, Op, run};

pub const META: ToolMeta = ToolMeta {
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
};

#[derive(Parser, Debug)]
#[command(name = "rsomics-tsv-crosstab", version, about, long_about = None, disable_help_flag = true)]
pub struct Cli {
    /// The two 1-based fields to cross-tabulate: rows,columns (e.g. 1,2).
    #[arg(short = 'g', long = "fields", value_name = "X,Y")]
    fields: String,

    /// Aggregate cells with this operation instead of counting co-occurrences.
    #[arg(long = "op", value_name = "OP")]
    op: Option<String>,

    /// 1-based field the aggregate reads (required with --op).
    #[arg(long = "value", value_name = "N")]
    value: Option<usize>,

    /// Sort input by the key fields first. Without it, only consecutive runs of
    /// a key are grouped (datamash's default — input is assumed pre-sorted).
    #[arg(short = 's', long = "sort")]
    sort: bool,

    /// Field separator (single byte). `-t` is reserved for thread count.
    #[arg(long = "field-separator", default_value = "\t")]
    field_separator: String,

    /// First input line is column headers and is dropped.
    #[arg(long = "header-in")]
    header_in: bool,

    /// Value placed in empty cells.
    #[arg(long = "filler", default_value = "N/A")]
    filler: String,

    /// Separator between elements in `unique`/`collapse` cells.
    #[arg(short = 'c', long = "collapse-delimiter", default_value = ",")]
    collapse_delimiter: String,

    /// Input file ("-" for stdin).
    #[arg(short = 'f', long = "file", default_value = "-")]
    input: PathBuf,

    #[command(flatten)]
    pub common: CommonFlags,
}

impl Cli {
    fn build_config(&self) -> Result<Config> {
        let delim = single_byte(&self.field_separator, "field separator")?;
        let cdelim = single_byte(&self.collapse_delimiter, "collapse delimiter")?;

        let (x, y) = parse_pair(&self.fields)?;

        let agg = match (&self.op, self.value) {
            (None, None) => None,
            (Some(name), Some(v)) => {
                let op = Op::parse(name).ok_or_else(|| {
                    RsomicsError::InvalidInput(format!("unknown operation '{name}'"))
                })?;
                Some((op, v))
            }
            (Some(_), None) => {
                return Err(RsomicsError::InvalidInput(
                    "--op requires --value <field>".into(),
                ));
            }
            (None, Some(_)) => {
                return Err(RsomicsError::InvalidInput(
                    "--value given without --op".into(),
                ));
            }
        };

        Ok(Config {
            delim,
            x,
            y,
            agg,
            sort: self.sort,
            header_in: self.header_in,
            filler: self.filler.clone().into_bytes(),
            collapse_delim: cdelim,
        })
    }
}

fn single_byte(s: &str, what: &str) -> Result<u8> {
    let b = s.as_bytes();
    if b.len() != 1 {
        return Err(RsomicsError::InvalidInput(format!(
            "{what} must be a single byte"
        )));
    }
    Ok(b[0])
}

fn parse_pair(spec: &str) -> Result<(usize, usize)> {
    let parts: Vec<&str> = spec.split(',').collect();
    if parts.len() != 2 {
        return Err(RsomicsError::InvalidInput(
            "crosstab requires exactly 2 fields, e.g. --fields 1,2".into(),
        ));
    }
    let x = field_index(parts[0])?;
    let y = field_index(parts[1])?;
    Ok((x, y))
}

fn field_index(s: &str) -> Result<usize> {
    let n: usize = s
        .trim()
        .parse()
        .map_err(|_| RsomicsError::InvalidInput(format!("invalid field number '{s}'")))?;
    if n == 0 {
        return Err(RsomicsError::InvalidInput(
            "field numbers are 1-based".into(),
        ));
    }
    Ok(n)
}

impl Tool for Cli {
    fn meta() -> ToolMeta {
        META
    }
    fn common(&self) -> &CommonFlags {
        &self.common
    }
    fn execute(self) -> Result<()> {
        let cfg = self.build_config()?;
        let mut out = std::io::stdout().lock();
        run(&self.input, &cfg, &mut out)
    }
}

pub static HELP: HelpSpec = HelpSpec {
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    tagline: "Cross-tabulate a delimited file into a contingency table.",
    origin: Some(Origin {
        upstream: "GNU datamash",
        upstream_license: "GPL-3.0-or-later",
        our_license: "MIT OR Apache-2.0",
        paper_doi: None,
    }),
    usage_lines: &["--fields <X,Y> [--op <op> --value <N>] [--field-separator <sep>] [-f <file>]"],
    sections: &[Section {
        title: "OPTIONS",
        flags: &[
            FlagSpec {
                short: Some('g'),
                long: "fields",
                aliases: &[],
                value: Some("<X,Y>"),
                type_hint: Some("two 1-based fields"),
                required: true,
                default: None,
                description: "Fields to cross-tabulate: rows,columns (e.g. 1,2).",
                why_default: None,
            },
            FlagSpec {
                short: None,
                long: "op",
                aliases: &[],
                value: Some("<op>"),
                type_hint: Some("count|sum|mean|min|max|median|…"),
                required: false,
                default: Some("count"),
                description: "Aggregate cells with this op instead of counting.",
                why_default: None,
            },
            FlagSpec {
                short: None,
                long: "value",
                aliases: &[],
                value: Some("<N>"),
                type_hint: Some("1-based field"),
                required: false,
                default: None,
                description: "Field the aggregate reads (required with --op).",
                why_default: None,
            },
            FlagSpec {
                short: Some('s'),
                long: "sort",
                aliases: &[],
                value: None,
                type_hint: None,
                required: false,
                default: None,
                description: "Sort by key fields first; otherwise only consecutive runs group.",
                why_default: None,
            },
            FlagSpec {
                short: None,
                long: "field-separator",
                aliases: &[],
                value: Some("<sep>"),
                type_hint: Some("single byte"),
                required: false,
                default: Some("tab"),
                description: "Field separator (datamash's -t; -t here is thread count).",
                why_default: Some("TSV is the bioinformatics default"),
            },
            FlagSpec {
                short: None,
                long: "filler",
                aliases: &[],
                value: Some("<s>"),
                type_hint: None,
                required: false,
                default: Some("N/A"),
                description: "Value placed in empty cells.",
                why_default: None,
            },
            FlagSpec {
                short: None,
                long: "header-in",
                aliases: &[],
                value: None,
                type_hint: None,
                required: false,
                default: None,
                description: "Drop the first input line (column headers).",
                why_default: None,
            },
            FlagSpec {
                short: Some('c'),
                long: "collapse-delimiter",
                aliases: &[],
                value: Some("<X>"),
                type_hint: None,
                required: false,
                default: Some(","),
                description: "Separator inside unique/collapse cells.",
                why_default: None,
            },
        ],
    }],
    examples: &[
        Example {
            description: "Count co-occurrences of field 1 (rows) and field 2 (columns)",
            command: "rsomics-tsv-crosstab --fields 1,2 -f data.tsv",
        },
        Example {
            description: "Sum field 3 per (field-1, field-2) cell",
            command: "rsomics-tsv-crosstab --fields 1,2 --op sum --value 3 -f data.tsv",
        },
    ],
    json_result_schema_doc: None,
};

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn cli_debug_assert() {
        Cli::command().debug_assert();
    }
}
