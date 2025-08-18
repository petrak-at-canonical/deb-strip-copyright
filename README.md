## Why not `debian-copyright`?

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
