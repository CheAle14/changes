# Changes

Detect whether a file or directory has changed through calculating a hash.

Intended for use in build scripts to skip building things when they have not changed.

For directories, obeys gitignores in the directory and any child directories.
Currently does not obey gitignores in any parent directories from the path given.

## Hash files

When `get_changes` is called on a file path, the hash is stored in `.<filename>.changes.hash`.  
When it is called on a directory, the hash is stored in `.changes.hash` within the directory.
