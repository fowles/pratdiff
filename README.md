A diff tool that provides line and token level colorization.

![Sample Code Diff](https://raw.githubusercontent.com/fowles/pratdiff/main/docs/sample.png)

Based on the [Patience Diff
algorithm](https://bramcohen.livejournal.com/73318.html) described by Bram
Cohen and then expanded upon by James Coglan in two blogs posts
([algorithm](https://blog.jcoglan.com/2017/09/19/the-patience-diff-algorithm/)
and
[implementation](https://blog.jcoglan.com/2017/09/28/implementing-patience-diff/)).

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
