//! Type-safe specialization of generic Debian control files
//! to `debian/copyright` syntax.
//!
//! https://www.debian.org/doc/packaging-manuals/copyright-format/1.0

use std::{collections::HashSet, path::Path, str::FromStr};

use eyre::{Context, eyre};
use log::{debug, trace};

use crate::{deb822::Deb822File, glob::Glob};

/// Specialization of [`Deb822File`] that throws away most of the information
/// except for all the file exclusions
#[derive(Clone, Debug)]
pub struct CopyrightFile {
  /// Duplicated merged hashset of all the normal
  /// excluded files.
  literal_excludes: HashSet<String>,
  glob_excludes: Vec<Glob>,
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
      literal_excludes: HashSet::new(),
      glob_excludes: Vec::new(),
    };

    for deb_stanza in deb.stanzas.into_iter() {
      if let Some(fex) = deb_stanza.fields.get("Files-Excluded") {
        for form in fex.iter_lines().cloned() {
          let glob = Glob::from_str(&form)
            .wrap_err_with(|| eyre!("while parsing glob string {:?}", &form))?;
          if let Some(lit) = glob.as_single_literal() {
            out.literal_excludes.insert(lit.to_owned());
          } else {
            out.glob_excludes.push(glob);
          }
        }
      }
    }

    debug!(
      "specialized CopyrightFile with {} glob excludes and {} literal excludes",
      out.glob_excludes.len(),
      out.literal_excludes.len()
    );
    Ok(out)
  }

  pub fn is_path_excluded<P: AsRef<Path>>(&self, p: P) -> bool {
    let p = p.as_ref();

    let path_str = p.to_string_lossy();
    if self.literal_excludes.contains(&*path_str) {
      false
    } else {
      let any_match = self
        .glob_excludes
        .iter()
        .any(|glob| glob.matches(&*path_str));
      any_match
    }
  }
}

impl FromStr for CopyrightFile {
  type Err = eyre::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let deb = Deb822File::from_str(s)?;
    Self::new(deb)
  }
}
