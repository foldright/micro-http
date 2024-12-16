# Publishing Guide for micro-http Workspace

This guide details the process of publishing the micro-http workspace packages to crates.io.

## Prerequisites

### 1. Install cargo-workspaces

`cargo-workspaces` is a powerful tool for managing Rust workspace versioning. Install it using:

```bash
cargo install cargo-workspaces
```

Verify installation:
```bash
cargo ws --version
```

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

Use cargo-workspaces to manage versions:
```bash
# List current versions
cargo ws list

# Bump versions (replace <TYPE> with major/minor/patch)
cargo ws version <TYPE> --no-git-push
```

### 2. Review Publishing Order

Check the publishing order of crates:
```bash
cargo ws plan
```

This will show you the dependency order for publishing. Note this order for the next step.

### 3. Dry Run Publishing

For each crate in the correct order (from cargo ws plan), perform a dry run:

```bash
# Replace ${crate} with the crate name
cargo publish --dry-run -p ${crate}
```

### 4. Actual Publishing

If all dry runs succeed, publish each crate in order:

```bash
# Replace ${crate} with each crate name in order
cargo publish -p ${crate}
```

Example publishing sequence (adjust according to your plan output):
```bash
cargo publish -p micro-http
cargo publish -p micro-web
# ... other crates in order
```

### 5. Git Management

After successful publishing:
```bash
# Push version changes
git push origin main

# Push tags
git push origin --tags
```

## Post-release Verification

1. **Package Verification**
   - Verify packages are listed on [crates.io](https://crates.io)
   - Check documentation generation on [docs.rs](https://docs.rs)
   - Ensure all published versions are accessible

2. **Version Verification**
   ```bash
   # Verify current versions
   cargo ws list
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

- [cargo-workspaces documentation](https://github.com/pksunkara/cargo-workspaces)
- [crates.io documentation](https://doc.rust-lang.org/cargo/reference/publishing.html)
- File issues on the project's GitHub repository

## Additional Commands

Useful cargo-workspaces commands for version management:
```bash
# List all workspace crates
cargo ws list

# View changed crates
cargo ws changed

# View publishing order
cargo ws plan
```

