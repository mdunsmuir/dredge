# dredge

[![Crates.io Status](https://img.shields.io/crates/v/dredge.svg)](https://crates.io/crates/dredge)

A utility for inspecting disk usage in directory trees.

## Usage

    dredge <directory to inspect>
    
`k` and `PgUp` go up, `j` and `PgDn` go down, `l` descends one level down into the selected
directory, and `h` goes one level up. `q` quits.

`d` deletes a file or directory; you will see a `(y/N)` prompt each time you use this function.
The deletion is recursive, i.e. deletion of a directory will delete all its
contents. Symbolic links will be deleted without following.
The delete function will **always** delete something if you have the permissions
to do so, e.g. if a file or directory is write protected but owned by you, it will
be deleted just like any other file. Directories containing write protected files
will similarly be deleted with no special warning.

## Caveats

* Deletion of write-protected files, see above.
* `dredge` is pretty dumb. If it can't delete a file for any reason, it just
*won't*. The file won't disappear from `dredge`'s listing, but otherwise
you won't see any special feedback indicating that there was a failure.
* Continuing on the "`dredge` is dumb" theme, `dredge` will generally ignore
things it doesn't understand. It just won't show them to you, or you'll see
a zero byte 'file' that can't be deleted.
* `dredge` won't follow symbolic links. It just sees them as regular files,
though it will show you the link targets.
* `dredge` doesn't account for multiple hard links pointing to the same inode,
i.e. it will count that disk usage twice.
* `dredge` will happily cross filesystem boundaries without telling you.
* `dredge` loads the target directory tree into memory on startup, and
from that point onwards it never attempts to check the consistency of its
model against the real thing. If you make changes outside of `dredge` and
don't restart it, you won't see those changes (though deletion operations
may fail if the files they target no longer exist).
* `dredge` is ~~fairly~~ very wasteful in its use of memory. Memory's cheap, right?

## Disclaimer

`dredge` is immature software written as a hobby project to learn Rust 
by someone (me) for whom the description
"does not possess guru-level understanding of file systems" is a severe
understatement of the actual level of ignorance involved. Though I don't 
think anyone will actually use it, I am releasing
it because I've personally found it useful. I make no guarantees about it being
bug-free, reasonably performant, correct in its presentation of data, or
anything else.

Note that `ncdu` is a similar program with a much higher level of maturity,
more cool features, and a larger user base. You should probably just use `ncdu`
instead, for now.
