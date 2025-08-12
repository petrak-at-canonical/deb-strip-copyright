mod parse_copyright;

use std::{
  path::{Path, PathBuf},
  str::FromStr,
};

use clap::{Parser, Subcommand, command};

use crate::parse_copyright::Deb822File;

#[derive(Parser)]
#[command(version, propagate_version = true)]
struct Args {
  #[command(subcommand)]
  subcommand: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
  /// Parse a file in Deb822 format, and dump the AST to stdout.
  /// This is mostly for debugging.
  #[command(name = "parse-deb")]
  ParseDeb822 { path: PathBuf },
}

fn main() -> eyre::Result<()> {
  let cli = Args::parse();

  match cli.subcommand {
    Subcommands::ParseDeb822 { path } => {
      let file = std::fs::read_to_string(path)?;
      let ast = Deb822File::from_str(&file)?;
      println!("{:#?}", &ast);
    }
  }

  Ok(())
}
