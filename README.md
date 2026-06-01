# rsomics-tsv-crosstab

Cross-tabulate a delimited file: pivot a long table into a wide contingency
table. Byte-for-byte compatible with `datamash crosstab`.

```sh
# Count co-occurrences of field 1 (rows) and field 2 (columns)
rsomics-tsv-crosstab --fields 1,2 -f data.tsv

# Aggregate a third field per cell instead of counting
rsomics-tsv-crosstab --fields 1,2 --op sum --value 3 -f data.tsv
```

Given

```
ctrl	liver
ctrl	liver
treat	kidney
```

`--fields 1,2` produces

```
	kidney	liver
ctrl	N/A	2
treat	1	N/A
```

## Behaviour

- `--fields X,Y` selects the two 1-based fields: `X` becomes the row labels,
  `Y` the column labels. The top-left corner cell is empty.
- Row and column labels are sorted by **byte order** (lexicographic, not
  numeric — `1`, `10`, `2`), matching datamash.
- Like datamash, only **consecutive runs** of a key are grouped: the input is
  assumed pre-sorted, and on a non-adjacent reoccurrence the first run's cell
  value is kept. Pass `--sort` (`-s`) to sort by the key fields first and
  aggregate the whole input (datamash's `-s`).
- Default cells count co-occurrences. `--op <name> --value <field>` aggregates
  that field instead: `sum`, `min`, `max`, `absmin`, `absmax`, `range`,
  `mean`, `median`, `q1`, `q3`, `iqr`, `pstdev`, `sstdev`, `pvar`, `svar`,
  `first`, `last`, `unique`, `collapse`, `countunique`, `count`.
- Numeric results print with C `%.14g` formatting, as datamash does. The
  integer/min/max/textual ops are byte-identical; datamash accumulates the
  statistical ops (`mean`, `pstdev`, …) in x86 80-bit `long double`, so on
  large inputs those can differ from our IEEE-`f64` result in the last
  significant digit (a non-portable datamash artifact).
- Empty cells are filled with `--filler` (default `N/A`).
- `unique` and `collapse` join elements with `--collapse-delimiter` (default
  `,`); `unique` sorts and dedups, `collapse` keeps input order.
- Default separator is TAB; `--field-separator` changes it (`-t` is reserved
  for the thread count). `--header-in` drops the first line.
- A row missing a requested field is an error, matching datamash.

## Origin

This crate is an independent Rust reimplementation of `datamash crosstab`
based on:

- The public `datamash` manual and `--help` output.
- Black-box behaviour testing against the upstream binary (label sort order,
  the `N/A` filler, the empty corner cell, `%.14g` numeric formatting, and the
  aggregate operations).

No source code from the GPL upstream was used as reference during
implementation. Test fixtures are independently generated.

License: MIT OR Apache-2.0.
Upstream credit: GNU datamash <https://www.gnu.org/software/datamash/> (GPL-3.0-or-later).
