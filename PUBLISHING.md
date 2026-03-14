# Publishing Checklist

## One-time setup

- Ensure you have a crates.io account.
- Login locally:

```sh
cargo login <token>
```

## Before publish

1. Verify metadata in `Cargo.toml`:
   - `name`, `version`, `description`, `license`, `readme`
2. Update `CHANGELOG.md` with the release date and notes.
3. Ensure README examples compile or are accurate.
4. Run tests:

```sh
cargo test
```

5. Dry run packaging:

```sh
cargo package
```

## Publish

From the crate directory:

```sh
cargo publish
```

## After publish

- Tag the release in git.
- Announce the release and update any docs referencing the version.
