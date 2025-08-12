//! Type-safe conversion of generic Debian control files
//! to `debian/copyright` syntax.

use std::{collections::HashSet, path::Path, str::FromStr};

use eyre::OptionExt;

use crate::deb822::Deb822File;

#[derive(Clone, Debug)]
pub struct CopyrightFile {
  /// List of stanzas
  stanzas: Vec<CopyrightStanza>,
  /// Duplicated merged hashset of all the normal
  /// excluded files.
  all_normal_excludes: HashSet<String>,
}

impl CopyrightFile {
  /// Pull the relevant information out of the deb file.
  ///
  /// At the moment, because this program is meant for excluding
  /// files and nothing else, stanzas without any copyright
  /// information are not put into `self`.
  pub fn new(deb: Deb822File) -> eyre::Result<Self> {
    // this is very hard to write as an iterator train
    let stanzas = deb
      .stanzas
      .into_iter()
      .filter_map(|deb_stanza| {
        if let Some(fex) = deb_stanza.fields.get("Files-Excluded") {
          let usn = deb_stanza
            .fields
            .get("Upstream-Name")
            .and_then(|f| f.same_line_value.clone());
          let mut cs = CopyrightStanza {
            upstream_name: usn,
            files_excluded_normal: HashSet::new(),
            files_excluded_wildcard: Vec::new(),
          };
          for form in fex.iter_lines().cloned() {
            if form.contains('*') {
              cs.files_excluded_wildcard.push(form);
            } else {
              cs.files_excluded_normal.insert(form);
            }
          }
          Some(cs)
        } else {
          None
        }
      })
      .collect();

    let mut out = CopyrightFile {
      stanzas,
      all_normal_excludes: HashSet::new(),
    };
    out.recalculate_excludes();
    Ok(out)
  }

  fn recalculate_excludes(&mut self) {
    self.all_normal_excludes.clear();
    self.all_normal_excludes.extend(
      self
        .stanzas
        .iter()
        .flat_map(|stanza| stanza.files_excluded_normal.iter().cloned()),
    );
  }

  pub fn is_path_excluded<P: AsRef<Path>>(&self, p: P) -> bool {
    let p = p.as_ref();

    let path_str = p.to_string_lossy();
    if self.all_normal_excludes.contains(&*path_str) {
      false
    } else {
      let any_match = self.stanzas.iter().any(|stanza| {
        // TODO: match wildcard exclusions
        // They use less powerful rules than `glob` crate
        false
      });
      !any_match
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

#[derive(Clone, Debug)]
pub struct CopyrightStanza {
  /// Not actually used, but handy for debugging.
  pub upstream_name: Option<String>,
  /// Lines in `Files-Excluded` that do *not* have any
  /// wildcards in them.
  pub files_excluded_normal: HashSet<String>,
  /// Lines in `Files-Excluded` that *DO* have
  /// wildcards in them.
  pub files_excluded_wildcard: Vec<String>,
}
