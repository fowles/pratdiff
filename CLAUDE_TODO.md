## Cluster mode: IoError events are silently dropped

In `--cluster` mode, `group_into_clusters()` only processes `TextDiff` events.
`IoError`, `Binary`, and `TypeMismatch` events from `walk_file_pairs()` are
ignored. Consider printing or counting them, especially `IoError`.
