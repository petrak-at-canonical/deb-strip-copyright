//! Parse debian control files.
//!
//! The syntax is documented here:
//! https://www.debian.org/doc/debian-policy/ch-controlfields
//!
//! There is a crate for parsing these, but it is rather poor
//! IMHO.
//! The source is here: https://github.com/jelmer/deb822-rs

pub mod copyright;

use std::{collections::HashMap, str::FromStr};

use eyre::{Context, OptionExt, eyre};

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
  // The docs are silent on whether duplicate field names are allowed.
  // For simplicity I will make this a HashMap
  /// Maps a field name to the field data.
  pub fields: HashMap<String, Field>,
}

#[derive(Debug, Clone)]
pub struct Field {
  pub same_line_value: Option<String>,
  pub list_values: Vec<String>,
}

impl Field {
  /// Convenience function that chains over `same_line_value`
  /// and `list_values`
  pub fn iter_lines(&self) -> impl Iterator<Item = &String> + '_ {
    self.same_line_value.iter().chain(self.list_values.iter())
  }
}

impl FromStr for Deb822File {
  type Err = eyre::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let meta = ParseMeta { source: s };

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
      let (next_lines_slice, stanza) = meta.eat_stanza(lines_slice)?;
      stanzas.push(stanza);
      lines_slice = next_lines_slice;
    }

    Ok(Deb822File { stanzas })
  }
}

struct ParseMeta<'source> {
  source: &'source str,
}

impl<'source> ParseMeta<'source> {
  /// If `fragment` is within this string, return the row and column
  /// that it starts at. Note these are 0-indexed.
  fn find_fragment_row_col(&self, fragment: &str) -> Option<(usize, usize)> {
    // This is the evil part. It could be done with string
    // searching instead, but i like this solution.
    // even if it's really evil
    let self_start = self.source.as_ptr() as usize;
    let frag_start = fragment.as_ptr() as usize;

    let slice_ok = self_start <= frag_start
      && (self_start + self.source.len()) >= (frag_start + fragment.len());
    if !slice_ok {
      None
    } else {
      // Ok we know that fragment comes from self.
      let frag_offset = frag_start - self_start;
      let row_col = self
        .source
        .char_indices()
        .filter_map(
          |(byte_idx, ch)| {
            if ch == '\n' { Some(byte_idx) } else { None }
          },
        )
        .take_while(|byte_idx| *byte_idx < frag_offset)
        .enumerate()
        .last()
        .map(|(nl_count, last_nl_char_idx)| {
          (nl_count + 1, frag_offset - last_nl_char_idx)
        })
        .unwrap_or((0, 0));
      Some(row_col)
    }
  }

  fn eyre(&self, fragment: &str, error: eyre::Error) -> eyre::Error {
    let row_col = if let Some((row, col)) = self.find_fragment_row_col(fragment)
    {
      format!("{}:{}", row + 1, col)
    } else {
      "?:?".to_string()
    };
    error.wrap_err(format!("at {} ({})", row_col, fragment))
  }

  fn eat_stanza<'a>(
    &self,
    mut lines: &'a [&'a str],
  ) -> eyre::Result<(&'a [&'a str], Stanza)> {
    let mut out = Stanza {
      fields: HashMap::new(),
    };

    while !lines.is_empty() {
      let (rest, field_name, field) = self.eat_field(lines)?;
      let prev = out.fields.insert(field_name.clone(), field);
      if let Some(prev) = prev {
        return Err(self.eyre(
          &lines[0],
          eyre!(
            "duplicate key {} (previous had value {:?})",
            &field_name,
            &prev
          ),
        ));
      }
      lines = rest;

      // After each field, if the next line is a newline, go to
      // the next stanza
      if let Some(line) = lines.get(0)
        && line.trim().is_empty()
      {
        let nl_count = lines
          .iter()
          .take_while(|line| line.trim().is_empty())
          .count();
        lines = &lines[nl_count..];
        break;
      }
    }

    Ok((lines, out))
  }

  /// Return the parsed field and the remainder of uninteresting lines.
  fn eat_field<'a>(
    &self,
    lines: &'a [&'a str],
  ) -> eyre::Result<(&'a [&'a str], String, Field)> {
    let (top_line, rest_lines) = lines
      .split_first()
      .ok_or_eyre("ran out of lines (impossible?)")?;
    if top_line.starts_with(WHITESPACE) {
      return Err(self.eyre(
        top_line,
        eyre!("field header must not start with whitespace"),
      ));
    }

    let (field_name, oneline_value) = self.parse_field_oneliner(top_line)?;
    let (rest_lines, list_values) = self.eat_multiline_field_lines(rest_lines);
    Ok((
      rest_lines,
      field_name,
      Field {
        same_line_value: oneline_value,
        list_values,
      },
    ))
  }
  ///
  /// Try to read the header line of a field.
  ///
  /// Return (`key`, `oneline_value`). If `oneline_value` is `None`,
  /// it is a multiline value.
  fn parse_field_oneliner(
    &self,
    rest: &str,
  ) -> eyre::Result<(String, Option<String>)> {
    let (field_name, rest) = rest.split_once(':').ok_or_else(|| {
      self.eyre(rest, eyre!("could not find `:` in field header line"))
    })?;
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
    &self,
    lines: &'a [&'a str],
  ) -> (&'a [&'a str], Vec<String>) {
    let out: Vec<_> = lines
      .iter()
      .map_while(|line| {
        let parsed = self.parse_multiline_field_line(line);
        // An error here just means this line was unsuccessful to parse.
        // If error, don't abort, just stop iteration
        parsed.ok()
      })
      .collect();
    // For each OK line, slice one off the input lines
    let remainder_lines = &lines[out.len()..];
    (remainder_lines, out)
  }

  fn parse_multiline_field_line(&self, line: &str) -> eyre::Result<String> {
    if !line.starts_with(WHITESPACE) {
      return Err(self.eyre(
        line,
        eyre!("multiline field lines must start with whitespace"),
      ));
    }
    Ok(line.trim_matches(WHITESPACE).to_owned())
  }
}
