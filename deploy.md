# Publishing Guide for micro-http Workspace

This guide details the process of publishing the micro-http workspace packages to crates.io.

## Prerequisites

### 1. Install cargo-workspaces

`cargo-workspaces` is a powerful tool for managing Rust workspace. Install it using:

```bash
cargo install cargo-workspaces
```

Verify installation:
```bash
cargo ws --version
```

Resources:
- [cargo-workspaces on GitHub](https://github.com/pksunkara/cargo-workspaces)
- [cargo-workspaces on crates.io](https://crates.io/crates/cargo-workspaces)

### 2. Crates.io Authentication

Ensure you're authenticated with crates.io:
```bash
cargo login
```

## Pre-release Steps

1. **Code Quality Checks**
   ```bash
   # Run all tests
   cargo test
   
   # Run clippy for additional checks
   cargo clippy
   
   # Ensure formatting is correct
   cargo fmt --all -- --check
   ```

2. **Workspace Status**
   ```bash
   # Check for changed crates since last release
   cargo ws changed
   
   # Review crates in publishing order
   cargo ws plan
   
   # Verify git status is clean
   git status
   ```

## Publishing Process

### 1. Version Management

Choose your versioning strategy:
```bash
# List current versions
cargo ws list

# Bump versions (replace <TYPE> with major/minor/patch)
cargo ws version <TYPE>
```

### 2. Dry Run

Always perform a dry run first:

```bash
cargo ws publish \
    --exact \                    
    --registry crates-io \       # Publish to crates.io
    --dry-run \                 # Perform dry run
    custom <NEW_VERSION>        # e.g., 0.1.0
```

### 3. Actual Release

If dry run succeeds, proceed with the actual release:

```bash
cargo ws publish \
    --exact \                   # Use exact version numbers
    --registry crates-io \      # Publish to crates.io
    --no-individual-tags \      # Create single tag instead of per-crate tags
    custom <NEW_VERSION>        # e.g., 0.1.0 / 0.1.0-alpha.6
```

## Post-release Verification

1. **Package Verification**
   - Verify packages are listed on [crates.io](https://crates.io)
   - Check documentation generation on [docs.rs](https://docs.rs)
   - Ensure all published versions are accessible

2. **Git Management**
   ```bash
   # Verify tags are created
   git tag -l
   
   # Push changes if --no-git-push was used
   git push origin main --tags
   ```

## Troubleshooting

### Common Issues

1. **Publishing Failures**
   - Verify crates.io authentication
   - Check for dependency version conflicts
   - Ensure version numbers are unique
   - Validate API token permissions

2. **Version Conflicts**
   ```bash
   # Check current versions
   cargo ws list
   
   # Review publishing order
   cargo ws plan
   ```

3. **Dependency Issues**
   - Ensure all dependencies are published
   - Check for yanked dependency versions
   - Verify compatibility of dependency versions

### Getting Help

- Review [cargo-workspaces documentation](https://github.com/pksunkara/cargo-workspaces)
- Check [crates.io documentation](https://doc.rust-lang.org/cargo/reference/publishing.html)
- File issues on the project's GitHub repository

## Additional Commands

Useful cargo-workspaces commands:
```bash
# List all workspace crates
cargo ws list

# Execute command in all crates
cargo ws exec -- <command>

# View changed crates
cargo ws changed

# View publishing order
cargo ws plan
```

