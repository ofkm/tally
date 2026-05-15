# tally

Source Code Line Analyzer inspired cloc.

Run it with no arguments to count the current directory:

```sh
tally
```

Pass one or more files or directories to count those instead:

```sh
tally src tests Cargo.toml
```

Use `--tree` to include directory totals under each language:

```sh
tally --tree
```

The binary prints totals by language: files, blank lines, comment lines, and code lines. It skips common heavy directories like `.git`, `node_modules`, `target`, and `dist`.
