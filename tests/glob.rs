use std::{ops::RangeBounds, str::FromStr};

use deb_strip_copyright::glob::Glob;
use eyre::bail;
use fastrand::Rng;

fn gen_string(rng: &mut Rng, size: impl RangeBounds<usize>) -> String {
  let sz = rng.usize(size);
  std::iter::repeat_with(|| rng.alphanumeric())
    .take(sz)
    .collect()
}

/// All globs without wildcards should match themself.
#[test]
fn identity_globs() -> eyre::Result<()> {
  let mut rng = Rng::with_seed(0o7604);
  for _ in 0..1000 {
    let string: String = gen_string(&mut rng, 2..20);

    let glob = Glob::from_str(&string)?;
    if !glob.matches(&string) {
      bail!(
        "glob {:?} did not match identity string {:?}",
        &glob,
        &string
      );
    }
  }

  Ok(())
}

/// Test that swapping characters in an identity glob
/// matches itself
#[test]
fn question_mark() -> eyre::Result<()> {
  let mut rng = Rng::with_seed(12345);

  for _ in 0..1000 {
    let base_str: String = gen_string(&mut rng, 2..20);
    let questionated: String = base_str
      .chars()
      .map(|c| if rng.bool() { '?' } else { c })
      .collect();

    let glob = Glob::from_str(&questionated)?;
    if !glob.matches(&base_str) {
      bail!(
        "{:?} did not match dequestionated str {:?}",
        &glob,
        &base_str
      );
    }
  }

  Ok(())
}

/// Make sure that `AAA*BBB` matches `AAACCCBBB`.
#[test]
fn single_star() -> eyre::Result<()> {
  let mut rng = Rng::with_seed(1234);

  for _ in 0..1000 {
    let front = gen_string(&mut rng, 2..10);
    let center = gen_string(&mut rng, 5..15);
    let back = gen_string(&mut rng, 2..10);

    let glob_str = format!("{}*{}", &front, &back);
    let test_str = format!("{}_{}_{}", &front, &center, &back);

    let glob = Glob::from_str(&glob_str)?;
    if !glob.matches(&test_str) {
      bail!("glob {:?} did not match string {:?}", &glob, &test_str);
    }
  }

  Ok(())
}

/// Make sure that a glob like `path/*` matches anything underneath
/// that path.
#[test]
fn match_paths_under() -> eyre::Result<()> {
  let mut rng = Rng::with_seed(5678);

  for _ in 0..1000 {
    let folder = gen_string(&mut rng, 5..15);
    let whatever = gen_string(&mut rng, 10..20);

    let glob_str = format!("{}/*", &folder);
    let test_str = format!("{}/{}", &folder, &whatever);

    let glob = Glob::from_str(&glob_str)?;
    if !glob.matches(&test_str) {
      bail!(
        "glob {:?} did not match folder-y string {:?}",
        &glob,
        &test_str
      );
    }
  }

  Ok(())
}

/// Check magic escaping.
#[test]
fn escape() -> eyre::Result<()> {
  for (glob, test) in &[
    ("hello\\*world", "hello*world"),
    ("hello\\*\\*\\*world", "hello***world"),
    ("hello\\?\\?_what\\?\\?", "hello??_what??"),
    ("what_the_f\\*\\*\\*\\?", "what_the_f***?"),
  ] {
    let glob = Glob::from_str(glob)?;
    if !glob.matches(test) {
      bail!("glob {:?} did not match {:?}", &glob, &test)
    }
  }

  Ok(())
}
