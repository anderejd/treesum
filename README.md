treesum
=======

Parallel SHA-1 checksum calculator for file trees.

 - Outputs **_sha1sum compatible format_** (`sha1sum --check FILE`).
 - Walks the input directory and all subdirs.
 - One worker thread per core, working on one file each.
 - Prints checksums to stdout.
 - Prints errors to stderr.
 - Returns 0 on success.

Changelog
---------

### 0.1.2
 - Updated dependencies.

### 0.1.1
 - Updated dependencies.

### 0.1.0
 - First release.
