//! Type-safe specialization of generic Debian control files
//! to `debian/copyright` syntax.
//!
//! https://www.debian.org/doc/packaging-manuals/copyright-format/1.0

use std::{path::Path, str::FromStr};

use eyre::{Context, eyre};
use log::info;

use crate::{deb822::Deb822File, glob::Glob};

/// Specialization of [`Deb822File`] that throws away most of the information
/// except for all the file exclusions.
#[derive(Clone, Debug)]
pub struct CopyrightFile {
  excludes: Vec<Glob>,
}

impl CopyrightFile {
  /// Pull the relevant information out of the deb file.
  ///
  /// At the moment, because this program is meant for excluding
  /// files and nothing else, stanzas without any copyright
  /// information are not put into `self`.
  pub fn new(deb: Deb822File) -> eyre::Result<Self> {
    let excludes: Result<Vec<_>, _> = deb
      .stanzas
      .iter()
      .filter_map(|stanza| stanza.fields.get("Files-Excluded"))
      .flat_map(|fex| fex.iter_lines())
      .flat_map(|line| line.split_ascii_whitespace())
      .filter_map(|glob_str| {
        let glob = Glob::from_str(&glob_str);
        match glob {
          Ok(glob) => {
            if !glob.is_empty() {
              Some(Ok(glob))
            } else {
              None
            }
          }
          ono @ Err(..) => Some(ono.wrap_err_with(|| {
            eyre!("while parsing glob string {:?}", &glob_str)
          })),
        }
      })
      .collect();
    let excludes = excludes?;
    info!(
      "specialized CopyrightFile, {} stanzas turned into {} globs",
      deb.stanzas.len(),
      excludes.len()
    );
    Ok(CopyrightFile { excludes })
  }

  /// Check if the given path is excluded.
  ///
  /// Note that this plays a little bit fast-and-loose with
  /// non-UTF8 paths. It first converts the path using
  /// [`Path::to_string_lossy`], which usually is good enough.
  ///
  /// If it becomes a problem I'll fix it.
  pub fn is_path_excluded<P: AsRef<Path>>(&self, p: P) -> bool {
    let p = p.as_ref();
    let path_str = p.to_string_lossy();
    self.excludes.iter().any(|glob| glob.matches(&*path_str))
  }
}

impl FromStr for CopyrightFile {
  type Err = eyre::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let deb = Deb822File::from_str(s)?;
    Self::new(deb)
  }
}
