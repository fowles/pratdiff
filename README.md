A diff tool that provides line and token level colorization.

![Sample Code Diff](https://raw.githubusercontent.com/fowles/pratdiff/main/docs/sample.png)

Based on the [Patience Diff
algorithm](https://bramcohen.livejournal.com/73318.html) described by Bram
Cohen and then expanded upon by James Coglan in two blogs posts
([algorithm](https://blog.jcoglan.com/2017/09/19/the-patience-diff-algorithm/)
and
[implementation](https://blog.jcoglan.com/2017/09/28/implementing-patience-diff/)).

# FAQ

## Why?

Cause I wanted a learning project and this seemed like a reasonable one.

## Did you learn anything interesting?

The way that token level diffing uses the same algorithm as the line level
diffing is pretty cool in my mind.  I didn't think going into it that I would
structure it that way and it all kinda just fell out.
