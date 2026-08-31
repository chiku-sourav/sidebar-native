# Contributing to sidevitals

Thank you for contributing! Please read these guidelines before opening a branch or pull request.

---

## Branch Naming Strategy

All branches **must** follow this pattern:

```
<type>/<short-description>
```

`<short-description>` must be **lowercase**, **hyphen-separated**, and **<= 50 characters**.

### Types

| Type | When to use |
|------|-------------|
| `feat/` | New feature or capability |
| `fix/` | Bug fix |
| `refactor/` | Code restructure with no behaviour change |
| `perf/` | Performance improvement |
| `chore/` | Build scripts, CI, dependency bumps, tooling |
| `docs/` | Documentation only |
| `test/` | Adding or improving tests |
| `release/` | Release preparation (version bump, changelog) |
| `hotfix/` | Critical fix applied directly from `main` |

### Examples

```
feat/gpu-usage-widget
fix/cpu-spike-on-idle
refactor/metrics-collection-layer
perf/reduce-d2d-draw-calls
chore/bump-windows-crate-0-59
docs/update-readme-screenshots
test/sidebar-layout-integration
release/v0-2-0
hotfix/crash-on-startup-win10
```

### Rules

- OK: `feat/dark-mode-toggle`
- OK: `fix/memory-leak-on-close`
- BAD: `feature/darkMode` (wrong prefix, camelCase)
- BAD: `FIX-memory-leak` (uppercase, missing slash)
- BAD: `my-branch` (no type prefix)
- BAD: `feat/` (empty description)

> The branch naming convention is **automatically enforced** by CI on every pull request targeting `main`.

---

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short summary>
```

Examples:
```
feat(sidebar): add GPU usage widget
fix(cpu): prevent spike on idle poll
perf(d2d): reduce draw calls per frame
chore(deps): bump sysinfo to 0.34
```

---

## Pull Requests

- Target `main` for all changes.
- Keep PRs focused — one logical change per PR.
- Ensure `cargo test` and `cargo clippy` pass locally before opening a PR.

---

## Local Setup

```powershell
# Build debug binary
cargo build

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Build release binary
cargo build --release --target x86_64-pc-windows-msvc
```
