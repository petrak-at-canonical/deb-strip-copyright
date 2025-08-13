//! Strip the excludes out of an orig tarball.

use std::{io::BufReader, path::PathBuf, str::FromStr};

use clap::Args;
use eyre::{Context, eyre};
use indicatif::ProgressBar;
// i do not really like how this crate sets up its exports
use xz2::{bufread::XzDecoder, write::XzEncoder};

use crate::deb822::copyright::CopyrightFile;

/// Strip `Files-Excluded` from the orig tarball.
#[derive(Args)]
pub struct Strip {
  /// Original tar.xz file.
  #[arg(short, long)]
  input: PathBuf,
  /// Path to where the stripped tar.xz file should go.
  #[arg(short, long)]
  output: PathBuf,
  /// Path to the debian copyright file.
  /// [default: ./debian/copyright]
  #[arg(short, long)]
  debfile: Option<PathBuf>,
  /// If this is set, do not actually write the output file.
  #[arg(long)]
  dry_run: bool,
}

impl Strip {
  pub fn do_it(self) -> eyre::Result<()> {
    let copyright = {
      let path = self.debfile.unwrap_or(PathBuf::from("./debian/copyright"));
      let copyright_file =
        std::fs::read_to_string(&path).wrap_err_with(|| {
          eyre!("could not read copyright file at {}", path.display())
        })?;

      CopyrightFile::from_str(&copyright_file)
        .wrap_err(eyre!("could not parse copyright file"))?
    };

    let in_file = std::fs::File::options()
      .read(true)
      .open(&self.input)
      .wrap_err_with(|| {
        eyre!("could not open input file at {}", self.input.display())
      })?;
    let xz = XzDecoder::new(BufReader::new(in_file));
    let mut xz_tar_reader = tar::Archive::new(xz);

    let mut tar_xz_writer = if self.dry_run {
      None
    } else {
      let out_file = std::fs::File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&self.output)
        .wrap_err_with(|| {
          eyre!("could not open output file at {}", self.output.display())
        })?;
      // TODO is it yak shaving to allow custom compression amount
      let xz = XzEncoder::new(out_file, 6);
      Some(tar::Builder::new(xz))
    };

    // this is hard to write as an iterator train because of propogating errors
    let mut keep_count = 0;
    let mut total_count = 0;
    // I can't find a good way to see how much of the tar file I have read.
    let progress_spinner = ProgressBar::new_spinner();
    for entry in xz_tar_reader
      .entries()
      .wrap_err("could not read entries from input tarfile")?
    {
      let mut entry = entry.wrap_err("malformed entry in input tar file")?;

      let real_path = entry.path()?.into_owned();
      // tarfile paths for `foo-bar.tar.xz` start with `foo-bar/`
      // so skip that
      let checked_path: PathBuf = real_path.components().skip(1).collect();
      let exclude = copyright.is_path_excluded(&checked_path);
      if !exclude {
        keep_count += 1;
        if let Some(ref mut txzw) = tar_xz_writer {
          let mut header = entry.header().clone();
          txzw.append_data(&mut header, &real_path, &mut entry)?;
        }
      }

      total_count += 1;
      // Only print every so often because you can't read that fast anyways
      if total_count % 10 == 0 {
        progress_spinner.set_message(format!(
          "{} {}",
          if exclude { "excl" } else { "incl" },
          checked_path.display()
        ));
      }
    }

    if let Some(mut txzw) = tar_xz_writer {
      txzw.finish()?;
    }

    progress_spinner.finish_with_message(format!(
      "kept {}/{} entries from the archive",
      keep_count, total_count
    ));

    Ok(())
  }
}
