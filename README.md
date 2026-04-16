A diff tool that provides line and token level colorization.

![Sample Code Diff](https://raw.githubusercontent.com/fowles/pratdiff/main/docs/sample.png)

Based on the [Patience Diff
algorithm](https://bramcohen.livejournal.com/73318.html) described by Bram
Cohen and then expanded upon by James Coglan in two blogs posts
([algorithm](https://blog.jcoglan.com/2017/09/19/the-patience-diff-algorithm/)
and
[implementation](https://blog.jcoglan.com/2017/09/28/implementing-patience-diff/)).

# Usage

```
❯ pratdiff --help
Diff files with token level colorization

Usage:
    pratdiff [OPTIONS] <OLD_FILE> <NEW_FILE>
        Diff two files or directory trees.

    <diff tool> | pratdiff [OPTIONS]
        Rediff an existing diff, adding pratdiff goodness.

        Examples:
          git diff | pratdiff
          diff -u old.txt new.txt | pratdiff --cluster

Arguments:
  [OLD_FILE]  Path to old file, directory tree, or `-` for stdin
  [NEW_FILE]  Path to new file, directory tree, or `-` for stdin

Options:
  -c, --context <NUM>        Display NUM lines of unchanged context before and after changes [default: 3]
  -v, --verbose-paths        Print full paths instead of stripping a common prefix
      --color <COLOR>        [default: auto] [possible values: auto, always, never]
      --cluster              Group diffs into clusters by change signature
      --completions <SHELL>  The shell to generate the completions for [possible values: bash, elvish, fish, nushell, powershell, zsh]
  -h, --help                 Print help
  -V, --version              Print version
```

# FAQ

## How do I install `pratdiff`?

Use `cargo install pratdiff`.  You probably want to get `cargo` from
[`rustup`](https://www.rust-lang.org/tools/install) or
[`brew`](https://brew.sh/).

## How do I enable autocompletions?

The `--completions` flag takes a shell and outputs a completion script.

```bash
eval "$(pratdiff --completions=bash)"
```

```fish
pratdiff --completions=fish | source
```

## Why did you bother doing this?

Cause I wanted a learning project and this seemed like a reasonable one.

## Why did you name it `pratdiff`?

I wanted to insert an "r" into `patdiff`, and I kind of like "prat" as an oddly
out of date insult.

## Did you learn anything interesting?

The way that token level diffing uses the same algorithm as the line level
diffing is pretty cool in my mind.  I didn't think going into it that I would
structure it that way and it all kinda just fell out.

Also, I learned that the tiny extension on patience diff I made to use
non-unique lines if unique ones fail is a known algorithm called "histogram
diff".
