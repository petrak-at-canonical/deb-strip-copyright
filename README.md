# deb-strip-copyright

A work-in-process replacement for `mk-origtargz`.

`mk-origtargz`'s method of stripping the `Files-Excluded` section
is extremely slow: for each excluded line, it invokes `tar` from
the shell to remove that file.
This program instead pulls in a tar library, reads the input tar
file entry-by-entry, and writes all
the un-filtered files to the output tar file.

My computer has a rather slow hard drive, and I gave up waiting for
`mk-origtargz` to process rustc after some 16 hours.
This program can process rustc in about 4 minutes.

## Sample Usage

Eventually I hope to have this program (or a fork of it) accept
the same CLI interface as `mk-origtargz`.
For now, however, invoke it like this:

`deb-strip-copyright strip -i ../rustc-1.83.0-src.tar.xz -o ../rustc-1.83_1.83.0+dfsg0ubuntu1~bpo2.orig.tar.xz`

You will have to plug in the paths to the orig tarball and the properly-formatted output tarball yourself.
By default it will look for the copyright file at `./debian/copyright`.

Use `--help` for more information.
There are some other subcommands in there for debugging purposes.

## Why not use the `debian-copyright` crate?

The [`debian-copyright`](https://docs.rs/debian-copyright/0.1.28/debian_copyright/index.html)
crate can *parse* `Files-Excluded` fields, but it does not expose
the logic for turning the entries into globs and checking if
a given filepath is actually excluded or not.
Given that I have to write the glob logic myself, I would rather
write the entire thing myself, rather than trying to use half of
this library.

This does mean that this program does less syntax validation
than `debian-copyright`, so it may accept some malformed files
that `debian-copyright` does not. Caveat emptor.
