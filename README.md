# ncsum-rs

`ncsum-rs` is a command-line tool designed to maintain the integrity of a large number of files of the same type in a directory, such as documents or videos. The main executable is named `ncsum`, and it offers several subcommands to perform various operations on the files.

## Subcommands

### 1. `get-hash`

**Description:** Calculates and displays the hash of the provided files, similar to the behavior of the md5sum command.

**Usage:**

```bash
$ ncsum get-hash [FILE]...
```

### 2. `name`

**Description:** Renames a file to its hash and creates a separate file containing both the hash and the original file name. It also creates a .ncsum file that describes the original file.

**Usage:**

```bash
$ ncsum name [FILE]...
```

### 3. `rename`

**Description:** Takes a file with a .ncsum or .pncsum extension and uses it to restore the file to its original state, renaming it accordingly.

**Usage:**

```bash
$ ncsum rename [FILE]...
```

### 4. `check`

**Description:** Checks the integrity of a file described by a `.ncsum` or `.pncsum` file. Optionally, it can only display mismatches or separate them into a designated directory.

**Usage:**

```bash
$ ncsum check [FILE]...
```

**Options:**

  - `-o`, `--only-show-mismatches`: Only display files with hash mismatches.
  - `-s`, `--separate-mismatches`: Move files with mismatches to a separate directory.

### 5. `pack`

**Description:** Converts an existing file into an .pncsum packaged file, containing the original file and a corresponding .ncsum file that describes it.

## Installation

To use `ncsum-rs`, follow these steps:

1. Clone the repository: `git clone https://github.com/kevin4rb200116/ncsum-rs.git`
2. Navigate to the project directory: `cd ncsum`
3. Build the executable: `cargo build --release`
4. Run the executable: `./target/release/ncsum [SUBCOMMAND] [OPTIONS] [FILES]`

## Dependencies

* `clap`: Command line argument parsing.
* `md5`: MD5 hashing algorithm.
* `serde`: Serialization/deserialization library.
* `cpio`: CPIO archive handling library.
