# deploy creates

This document is about how to publish the micro-http workspaces to the creates.io.

## Check cargo-workspaces

First you need to install cargo-workspaces if it's not installed on your machine.

Check from [github](https://github.com/pksunkara/cargo-workspaces) or [crates.io](https://crates.io/crates/cargo-workspaces)

### Check cargo-workspaces in the path
```bash
cargo ws --version
```
If not in the path, install cargo-workspaces

```bash
cargo install cargo-workspaces
```

## Run Tests

Make sure  `cargo test` passed, and workspaces is clean (no git dirty files)

## Dry run 

```bash
cargo ws publish  --exact --no-git-push --no-individual-tags  --dry-run  custom ${newVersion}
```

## Publish

cargo ws publish  --exact --no-individual-tags custom ${newVersion}



