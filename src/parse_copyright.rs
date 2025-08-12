//! Parse d/copyright files.
//!
//! The syntax is documented here:
//! https://www.debian.org/doc/debian-policy/ch-controlfields
//!
//! There is a crate for parsing these, but it is rather poor
//! IMHO.
//! The source is here: https://github.com/jelmer/deb822-rs
//!
//! This may be later expanded into a whole deb822 parser,
//! but for now it only understands Copyright files.

use std::{collections::HashSet, path::Path, str::FromStr};

use eyre::{Context, OptionExt, eyre};

pub struct CopyrightFile {
  /// List of stanzas
  pub stanzas: Vec<CopyrightStanza>,
  /// Duplicated merged hashset of all the normal
  /// excluded files.
  all_normal_excludes: HashSet<String>,
}

impl CopyrightFile {
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

pub struct CopyrightStanza {
  /// Not actually used, but handy for debugging.
  pub upstream_name: String,
  /// Lines in `Files-Excluded` that do *not* have any
  /// wildcards in them.
  pub files_excluded_normal: HashSet<String>,
  /// Lines in `Files-Excluded` that *DO* have
  /// wildcards in them.
  pub files_excluded_wildcard: Vec<String>,
}

// Parsing.
// Before we enter any `eat` function, comment lines are stripped.
// (Just easier that way).

/// Non-newline whitespaces. The stdlib function `trim_left`
/// and friends consider newlines to be whitespace.
const WHITESPACE: &[char] = &[' ', '\t'];

#[derive(Debug, Clone)]
pub struct Deb822File {
  stanzas: Vec<Stanza>,
}

#[derive(Debug, Clone)]
pub struct Stanza {
  pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
  pub key: String,
  pub data: FieldData,
}

#[derive(Debug, Clone)]
pub enum FieldData {
  OneLine(String),
  List(Vec<String>),
}

impl FromStr for Deb822File {
  type Err = eyre::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let lines: Vec<&str> = s
      .split('\n')
      .filter(|line| !line.trim_start().starts_with('#'))
      .collect();

    let mut lines_slice = lines.as_slice();
    let mut stanzas = Vec::new();
    while !lines_slice.is_empty() {
      // an error here does mean abort, because the only
      // way to safely end a file is to reach the end
      // cleanly w/o dangling whatever
      let (next_lines_slice, stanza) =
        eat_stanza(lines_slice).wrap_err_with(|| {
          let approx_error_line = lines.len() - lines_slice.len() + 1;
          format!("at line {approx_error_line}")
        })?;
      stanzas.push(stanza);
      lines_slice = next_lines_slice;
    }

    Ok(Deb822File { stanzas })
  }
}

fn eat_stanza<'a>(
  mut lines: &'a [&'a str],
) -> eyre::Result<(&'a [&'a str], Stanza)> {
  let mut out = Stanza { fields: Vec::new() };

  while !lines.is_empty() {
    let (rest, field) = eat_field(lines)?;
    out.fields.push(field);
    lines = rest;
  }

  // Make sure either the file is over, or there's a double-newline.
  match lines.split_first() {
    Some((must_be_nl, rest)) => {
      if !must_be_nl.is_empty() {
        return Err(eyre!("found dangling unknown content after a stanza"));
      } else {
        lines = rest;
      }
    }
    None => {}
  }

  Ok((lines, out))
}

/// Return the parsed field and the remainder of uninteresting lines.
fn eat_field<'a>(lines: &'a [&'a str]) -> eyre::Result<(&'a [&'a str], Field)> {
  let (top_line, rest_lines) =
    lines.split_first().ok_or_eyre("ran out of lines")?;
  if top_line.starts_with(WHITESPACE) {
    return Err(eyre!("field header must not start with whitespace"));
  }

  let (field_name, oneline_value) = parse_field_oneliner(top_line)
    .wrap_err_with(|| format!("while parsing field"))?;
  println!("{:?} :: {:?}", field_name, oneline_value);
  if let Some(oneline_value) = oneline_value {
    Ok((
      rest_lines,
      Field {
        key: field_name,
        data: FieldData::OneLine(oneline_value),
      },
    ))
  } else {
    let (rest_lines, data) = eat_multiline_field_lines(rest_lines);
    println!("{:?}", &data);
    Ok((
      rest_lines,
      Field {
        key: field_name,
        data: FieldData::List(data),
      },
    ))
  }
}

/// Try to read the header line of a field.
///
/// Return (`key`, `oneline_value`). If `oneline_value` is `None`,
/// it is a multiline value.
fn parse_field_oneliner(rest: &str) -> eyre::Result<(String, Option<String>)> {
  let (field_name, rest) = rest
    .split_once(':')
    .ok_or_eyre("could not find `:` in field line")?;
  let rest = rest.trim_start_matches(WHITESPACE);
  let oneline_value = if rest.is_empty() {
    None
  } else {
    Some(rest.to_owned())
  };
  Ok((field_name.to_owned(), oneline_value))
}

/// Consume lines until we find one that is not a valid value.
/// This function is infallible because it is legal to have
/// zero valid lines. (Although annoying. Please don't do that.)
fn eat_multiline_field_lines<'a>(
  lines: &'a [&'a str],
) -> (&'a [&'a str], Vec<String>) {
  let out: Vec<_> = lines
    .iter()
    .map_while(|line| {
      let parsed = parse_multiline_field_line(line);
      // An error here just means this line was unsuccessful to parse.
      // If error, don't abort, just stop iteration
      parsed.ok()
    })
    .collect();
  // For each OK line, slice one off the input lines
  let remainder_lines = &lines[out.len()..];
  (remainder_lines, out)
}

fn parse_multiline_field_line(line: &str) -> eyre::Result<String> {
  if !line.starts_with(WHITESPACE) {
    return Err(eyre!("multiline field lines must start with whitespace"));
  }
  Ok(line.trim_matches(WHITESPACE).to_owned())
}
