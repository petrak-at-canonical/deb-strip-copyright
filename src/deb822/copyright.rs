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
    // this is hard to write as an iterator train
    // because we need to shortcut-return the error possibly
    let mut out = CopyrightFile {
      excludes: Vec::new(),
    };

    for deb_stanza in deb.stanzas.iter() {
      if let Some(fex) = deb_stanza.fields.get("Files-Excluded") {
        for form in fex.iter_lines().cloned() {
          let glob = Glob::from_str(&form)
            .wrap_err_with(|| eyre!("while parsing glob string {:?}", &form))?;
          if !glob.is_empty() {
            out.excludes.push(glob);
          }
        }
      }
    }

    info!(
      "specialized CopyrightFile, {} stanzas turned into {} globs",
      deb.stanzas.len(),
      out.excludes.len()
    );
    Ok(out)
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
