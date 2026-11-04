# Release Process

Releases are cut from a Git tag and published to GitHub Releases. Nothing is
published to crates.io.

## How to Release

1. **Bump the version** — go to **Actions** → **Release** → **Run workflow** and
   select one of:
   - `bump-patch` (0.1.0 → 0.1.1) - bug fixes
   - `bump-minor` (0.1.0 → 0.2.0) - new features
   - `bump-major` (0.1.0 → 1.0.0) - breaking changes
2. **Review and merge** the auto-created `release/vX.Y.Z` pull request. It bumps
   the version in `Cargo.toml`, refreshes `Cargo.lock`, and regenerates
   `CHANGELOG.md`.
3. **Push the tag** on `main`:

   ```bash
   git switch main && git pull
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

4. **Done!** The tag push runs the release:

   - Checks the tag against the version in `Cargo.toml`
   - Creates the GitHub Release `vX.Y.Z`
   - Builds and uploads binaries for:
     - `x86_64-unknown-linux-gnu` (Linux x64, dynamically linked glibc)
     - `x86_64-unknown-linux-musl` (Linux x64, static musl)
     - `aarch64-unknown-linux-gnu` (Linux ARM64, dynamically linked glibc)
     - `aarch64-unknown-linux-musl` (Linux ARM64, static musl)
     - `x86_64-apple-darwin` (macOS x64)
     - `aarch64-apple-darwin` (macOS Apple Silicon)
     - `x86_64-pc-windows-msvc` (Windows x64)

Only tags that exactly match `vX.Y.Z` start a release. A tag such as
`vX.Y.Z-rc1` does not.

The tag must be pushed by a person or a token that is not `GITHUB_TOKEN`, since
GitHub does not start workflows for its own token.

## Build Binaries for Existing Release

If you need to rebuild binaries for an existing release:

1. **Go to Actions** → **Release** → **Run workflow**
2. **Select action:** `build-binaries`
3. **Enter version:** e.g., `0.1.1` (without the `v` prefix)
4. **Click "Run workflow"**

The tag `v0.1.1` must already exist. Linux binaries are built for both GNU and
musl targets, and their archive names use the matching target triples. The GNU
archives stay dynamically linked so that hostname lookup keeps working.

## Commit Message Convention

For meaningful changelogs, use conventional commits:

| Prefix | Category | Example |
|--------|----------|---------|
| `feat:` | Features | `feat: add export to JSON` |
| `fix:` | Bug Fixes | `fix: resolve crash on empty diff` |
| `docs:` | Documentation | `docs: update keybindings table` |
| `perf:` | Performance | `perf: optimize large file rendering` |
| `refactor:` | Refactor | `refactor: simplify state machine` |
| `test:` | Testing | `test: add integration tests` |
| `chore:` | Miscellaneous | `chore: update dependencies` |

## Manual Release (if needed)

```bash
# Update the version in Cargo.toml by hand, refresh the lockfile, then:
cargo generate-lockfile
git cliff --tag v0.2.0 -o CHANGELOG.md
git commit -am "chore(release): prepare v0.2.0"
git tag v0.2.0
git push origin main v0.2.0
```
