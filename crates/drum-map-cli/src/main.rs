//! Headless CLI for drum-midi-converter (SPEC §2.3, milestone M2).
//!
//! stdout carries report/list output only; every error goes to stderr with a
//! non-zero exit code.

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use drum_map::convert::ConvertOptions;
use drum_map::report::Report;

#[derive(Parser)]
#[command(
    name = "drum-midi-converter",
    version,
    about = "Convert drum MIDI files between drum-library note maps"
)]
struct Args {
    /// Input .mid file
    input: Option<PathBuf>,
    /// Source map id (e.g. ggd-invasion)
    #[arg(long)]
    from: Option<String>,
    /// Target map id (e.g. sd3)
    #[arg(long)]
    to: Option<String>,
    /// Output .mid file
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Print the conversion report (works with or without -o)
    #[arg(long)]
    report: bool,
    /// List available maps and exit
    #[arg(long)]
    list_maps: bool,
    /// What to do with unmapped notes (default: keep)
    #[arg(
        long,
        value_name = "KEEP|DROP|REASSIGN=N",
        value_parser = parse_unmapped,
        default_value = "keep"
    )]
    unmapped: drum_map::convert::Resolution,
}

/// Parse `keep`, `drop` or `reassign=N` (N must be 0-127) into a Resolution.
fn parse_unmapped(s: &str) -> Result<drum_map::convert::Resolution, String> {
    match s {
        "keep" => Ok(drum_map::convert::Resolution::Keep),
        "drop" => Ok(drum_map::convert::Resolution::Drop),
        _ => {
            let Some(n) = s.strip_prefix("reassign=") else {
                return Err(format!("expected keep, drop or reassign=N, got {s:?}"));
            };
            let v: u8 = n
                .parse()
                .map_err(|_| format!("reassign value must be a note number 0-127, got {n:?}"))?;
            if v > 127 {
                return Err(format!(
                    "reassign value must be a note number 0-127, got {v}"
                ));
            }
            Ok(drum_map::convert::Resolution::Reassign(v))
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.list_maps {
        return run_list_maps();
    }
    let input = args.input.as_deref().ok_or_else(|| {
        anyhow::anyhow!("no input file given: pass a .mid file, or use --list-maps")
    })?;
    let from = args
        .from
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing --from <map-id> (see --list-maps)"))?;
    let to = args
        .to
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing --to <map-id> (see --list-maps)"))?;

    let bytes = std::fs::read(input)
        .with_context(|| format!("failed to read input file {}", input.display()))?;
    let options = ConvertOptions {
        resolution: args.unmapped,
        ..Default::default()
    };
    let (out, report) = drum_map::midi::convert_by_id(&bytes, from, to, &options)
        .with_context(|| format!("failed to convert {} ({from} -> {to})", input.display()))?;

    match &args.output {
        Some(path) => {
            std::fs::write(path, &out)
                .with_context(|| format!("failed to write output file {}", path.display()))?;
            if args.report {
                print_report(&report);
            } else {
                println!("Wrote {} ({} bytes)", path.display(), out.len());
            }
        }
        None => {
            if args.report {
                print_report(&report);
            } else {
                println!(
                    "Dry run: {} bytes would be written (pass -o to write)",
                    out.len()
                );
            }
        }
    }
    Ok(())
}

fn run_list_maps() -> anyhow::Result<()> {
    let maps = drum_map::catalog::catalog().context("failed to load the embedded map catalog")?;
    let rows: Vec<Vec<String>> = maps
        .iter()
        .map(|m| {
            vec![
                m.id.clone(),
                m.name.clone(),
                m.vendor.clone(),
                m.notes.len().to_string(),
            ]
        })
        .collect();
    print_table(
        &["ID", "NAME", "VENDOR", "NOTES"],
        &rows,
        &[false, false, false, true],
    );
    Ok(())
}

fn print_report(report: &Report) {
    let rows: Vec<Vec<String>> = report
        .rows
        .iter()
        .map(|r| {
            let flags = match (r.lossy, r.unmapped) {
                (true, _) => "LOSSY",
                (_, true) => "UNMAPPED",
                _ => "",
            };
            vec![
                r.source_note.to_string(),
                r.source_name.clone(),
                r.hit_count.to_string(),
                r.target_note
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".into()),
                r.target_name.clone().unwrap_or_else(|| "-".into()),
                flags.into(),
            ]
        })
        .collect();
    print_table(
        &["SRC", "SRC NAME", "HITS", "DST", "DST NAME", "FLAGS"],
        &rows,
        &[true, false, true, true, false, false],
    );
    println!(
        "{} notes, {} lossy, {} unmapped",
        report.rows.len(),
        report.lossy().count(),
        report.unmapped().count(),
    );
}

/// Render a header row plus data rows with per-column alignment, padded with
/// two spaces between columns.
fn print_table(heads: &[&str], rows: &[Vec<String>], align_right: &[bool]) {
    let mut widths: Vec<usize> = heads.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let fmt = |cells: &[String]| {
        cells
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if align_right[i] {
                    format!("{:>width$}", c, width = widths[i])
                } else {
                    format!("{:<width$}", c, width = widths[i])
                }
            })
            .collect::<Vec<_>>()
            .join("  ")
    };
    let head_cells: Vec<String> = heads.iter().map(|h| h.to_string()).collect();
    println!("{}", fmt(&head_cells).trim_end());
    for row in rows {
        println!("{}", fmt(row).trim_end());
    }
}
