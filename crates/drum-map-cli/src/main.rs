//! Headless CLI for drum-midi-converter (SPEC §2.3, milestone M2).

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "drum-midi-converter",
    version,
    about = "Convert drum MIDI files between drum-library note maps"
)]
struct Args {
    /// Input .mid file
    input: Option<std::path::PathBuf>,
    /// Source map id (e.g. ggd-invasion)
    #[arg(long)]
    from: Option<String>,
    /// Target map id (e.g. sd3)
    #[arg(long)]
    to: Option<String>,
    /// Output .mid file
    #[arg(short, long)]
    output: Option<std::path::PathBuf>,
    /// List available maps and exit
    #[arg(long)]
    list_maps: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.list_maps {
        println!("(no maps yet — built-in catalog arrives in milestone M1)");
        return Ok(());
    }
    anyhow::bail!("conversion not implemented yet — see docs/roadmap.md, milestone M1/M2");
}
