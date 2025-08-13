mod deb822;
mod glob;
mod strip;

use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand, command};

use crate::{
  deb822::{Deb822File, copyright::CopyrightFile},
  glob::Glob,
  strip::Strip,
};

/// A replacement for mk-origtargz.
///
/// This program uses env_logger and its associated environment variables.
/// By default, it logs at the warn level.
/// You may specify -v any number of times to get more verbose information.
/// (-v: info, -vv: debug, -vvv: trace).
///
/// Exit Codes:
///
/// - 0 = success
///
/// - 1 = error/panic
///
/// Other subcommands may have their own exit codes.
#[derive(Parser)]
#[command(version, propagate_version = true)]
struct Cli {
  // this means that by default, only print warn!() and error!()
  // one -v means start printing info!(), -vv = debug!(), -vvv = trace!()
  #[command(flatten)]
  verbosity: clap_verbosity_flag::Verbosity<clap_verbosity_flag::WarnLevel>,
  #[command(subcommand)]
  subcommand: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
  #[command(name = "strip")]
  Strip(Strip),
  #[command(name = "debugs", subcommand)]
  DebugSubcommands(DebugSubcommands),
}

/// Debug tools for smoke-testing internal functions.
/// Probably not useful to the end user.
#[derive(Subcommand)]
enum DebugSubcommands {
  /// Parse a file in Deb822 format, and dump the AST to stdout.
  /// This is mostly for debugging.
  #[command(name = "parse-deb")]
  ParseDeb822 { path: PathBuf },
  /// Parse a file in Deb822 format, collect the data into specialized
  /// `debian/copyright` format, and dump the AST to stdout.
  /// This is mostly for debugging.
  #[command(name = "parse-copyright")]
  ParseCopyright { path: PathBuf },
  /// Parse a simplified Debian glob, and dump the AST or test it on
  /// a string.
  #[command(name = "glob")]
  ParseGlob {
    /// The glob to parse
    glob: String,
    /// Dump the AST to stdout?
    #[arg(short, long)]
    dump: bool,
    /// If provided, test if the glob matches this string.
    /// Prints `true` or `false` to stdout, and also sets
    /// the error code to `2` on failure.
    #[arg(short, long)]
    test: Option<String>,
  },
}

fn main() -> eyre::Result<()> {
  let cli = Cli::parse();
  env_logger::Builder::new()
    .filter_level(cli.verbosity.into())
    .init();

  match cli.subcommand {
    Subcommands::Strip(strip) => {
      strip.do_it()?;
    }
    Subcommands::DebugSubcommands(dbg) => match dbg {
      DebugSubcommands::ParseDeb822 { path } => {
        let file = std::fs::read_to_string(path)?;
        let ast = Deb822File::from_str(&file)?;
        println!("{:#?}", &ast);
      }
      DebugSubcommands::ParseCopyright { path } => {
        let file = std::fs::read_to_string(path)?;
        let ast = CopyrightFile::from_str(&file)?;
        println!("{:#?}", &ast);
      }
      DebugSubcommands::ParseGlob { glob, dump, test } => {
        let glob = Glob::from_str(&glob)?;
        if dump {
          println!("{:?}", &glob);
        }
        if let Some(test) = test {
          let ok = glob.matches(&test);
          println!("{}", ok);
          if !ok {
            std::process::exit(2);
          }
        }
      }
    },
  }

  Ok(())
}
