# Cutting a release

1. Update `CHANGELOG.md` in the obvious way.
2. Submit and push to main.
3. Add a version tag: `git tag 3.0.0`
4. Push the version tag: `git push --tags`
5. Relase it: `cargo publish`
