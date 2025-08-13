//! Simple Glob implementation that only allows `*`, `?`, and escapes.
//! This is in accordance with Debian copyright syntax.

use std::{fmt::Write, path::Path, str::FromStr};

use eyre::eyre;

/// Compiled glob, recognizing literal strings, `*`, and `?`
///
/// The documentation does not say whether the `*` is greedy or ungreedy.
/// This implementation assumes ungreedy. That is, it will match as few
/// characters as possible.
#[derive(Debug, Clone)]
pub struct Glob {
  segments: Vec<GlobSegment>,
}

#[derive(Clone)]
enum GlobSegment {
  Literal(String),
  Star,
  Question,
}

impl Glob {
  /// Check if this glob matches the given string.
  pub fn matches<S: AsRef<str>>(&self, s: S) -> bool {
    let mut s_slice = s.as_ref();

    let mut peeker = self.segments.iter().peekable();
    // Peekable's mutability doesn't generally agree with for loops
    while let Some(seg) = peeker.next() {
      match seg {
        GlobSegment::Literal(lit) => {
          if let Some(rest) = s_slice.strip_prefix(lit) {
            s_slice = rest;
          } else {
            return false;
          }
        }
        GlobSegment::Question => {
          let next_ch = s_slice.char_indices().next();
          if let Some((idx, _)) = next_ch {
            s_slice = &s_slice[idx..];
          } else {
            return false;
          }
        }
        GlobSegment::Star => {
          let Some(next_seg) = peeker.peek() else {
            // Else the glob ends in a star so match whatever
            return true;
          };
          let GlobSegment::Literal(next_lit) = next_seg else {
            // this should be forbidden by the FromStr impl
            panic!("cannot have a `*` followed by a wildcard!");
          };
          let Some(next_lit_start) = next_lit.chars().next() else {
            // this should also forbidden by the FromStr impl
            panic!("cannot have an empty Literal glob segment!");
          };
          if let Some(start_idx) = s_slice.find(next_lit_start) {
            // Slice away everything up to that point
            s_slice = &s_slice[start_idx..];
          } else {
            return false;
          }
        }
      }
    }

    s_slice.is_empty()
  }

  /// Return if this glob is empty.
  /// (It will never match anything.)
  pub fn is_empty(&self) -> bool {
    self.segments.is_empty()
  }

  /// If this glob is a single literal string, return it.
  /// (IE, this glob has no wildcards and can be matched via hashing.)
  pub fn as_single_literal(&self) -> Option<&str> {
    match self.segments.as_slice() {
      [] => None,
      [GlobSegment::Literal(lit)] => Some(lit.as_str()),
      _ => None,
    }
  }
}

impl FromStr for Glob {
  type Err = eyre::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut segments = Vec::new();

    let mut string = String::new();
    let mut escape_on = false;
    for c in s.chars() {
      if escape_on {
        if c == '\\' || c == '*' || c == '?' {
          string.push(c);
          escape_on = false;
        } else {
          return Err(eyre!("character {:?} cannot be escaped", c));
        }
      } else {
        match c {
          '\\' => {
            escape_on = true;
          }
          '*' | '?' => {
            if !string.is_empty() {
              segments.push(GlobSegment::Literal(string.clone()));
              string = String::new();
            }
            // A star cannot be followed by a wildcard.
            // ie: `*?`, `**` (how do we ungreedy match the star?)
            // but `???` or `?*` is OK.
            let prev = segments.last();
            let prev_ok = match prev {
              None | Some(GlobSegment::Literal(..)) => true,
              Some(GlobSegment::Star) => false,
              Some(GlobSegment::Question) => c != '*',
            };
            if !prev_ok {
              return Err(eyre!("cannot have a `*` next to another wildcard"));
            }

            segments.push(if c == '*' {
              GlobSegment::Star
            } else {
              GlobSegment::Question
            });
          }
          _ => {
            string.push(c);
          }
        }
      }
    }

    if !string.is_empty() {
      segments.push(GlobSegment::Literal(string));
    }

    Ok(Self { segments })
  }
}

impl std::fmt::Debug for GlobSegment {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      GlobSegment::Literal(l) => std::fmt::Debug::fmt(l, f),
      GlobSegment::Star => f.write_char('*'),
      GlobSegment::Question => f.write_char('?'),
    }
  }
}
