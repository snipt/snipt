# Release Process

This document outlines the process for creating a new release of Snipt.

## Versioning

We follow [Semantic Versioning](https://semver.org/) for our releases:

- **MAJOR** version for incompatible API changes
- **MINOR** version for backwards-compatible functionality
- **PATCH** version for backwards-compatible bug fixes

## Automatic Release Process

The project uses a GitHub Actions workflow to automate the release process. Here's how it works:

1. Tag a new version with Git: 
   ```bash
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   ```

2. Push the tag to GitHub:
   ```bash
   git push origin vX.Y.Z
   ```

3. The GitHub Actions workflow will automatically:
   - Generate a changelog using git-cliff
   - Create a GitHub release with the changelog
   - Build release binaries for multiple platforms:
     - Linux (x86_64)
     - macOS (x86_64 and ARM64)
     - Windows (x86_64)
   - Upload the release assets

## Commit Message Format

To ensure proper changelog generation, please follow the conventional commit format for your commit messages:

- `feat: add new feature` - for new features
- `fix: resolve issue` - for bug fixes
- `docs: update documentation` - for documentation updates
- `style: format code` - for code style changes (no functional changes)
- `refactor: improve code structure` - for code refactoring
- `perf: improve performance` - for performance improvements
- `test: add or update tests` - for test updates
- `chore: update dependencies` - for maintenance tasks

The changelog will be organized based on these commit types.

## Release Checklist

Before creating a new release, ensure the following:

1. All tests are passing
2. Documentation is up to date
3. Version numbers are updated in:
   - `Cargo.toml` files
   - Documentation
   - Any other relevant files

## Manual Release Steps (if needed)

If you need to create a release manually:

1. Build the release binaries:
   ```bash
   cargo build --release
   ```

2. Generate a changelog:
   ```bash
   cargo install git-cliff
   git cliff --latest > CHANGELOG.md
   ```

3. Create a new release on GitHub manually and upload the binaries.

## Release Assets

Each release includes the following assets:

- `snipt-vX.Y.Z-linux-x86_64.tar.gz` - Linux binary
- `snipt-vX.Y.Z-macos-x86_64.tar.gz` - macOS Intel binary
- `snipt-vX.Y.Z-macos-arm64.tar.gz` - macOS ARM binary
- `snipt-vX.Y.Z-windows-x86_64.zip` - Windows binary

## Post-Release

After a release is published:

1. Update the development version in `Cargo.toml` files
2. Create a new branch for the next development cycle
3. Update the changelog with a new "Unreleased" section

## Emergency Releases

For critical bug fixes that require an immediate release:

1. Create a new branch from the latest release tag
2. Apply the fix
3. Create a new patch version release
4. Cherry-pick the fix to the main branch

## Release Notes

Each release should include:

1. Summary of changes
2. Breaking changes (if any)
3. New features
4. Bug fixes
5. Known issues
6. Upgrade instructions (if needed) 