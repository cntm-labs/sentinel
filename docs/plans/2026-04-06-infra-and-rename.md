# Infrastructure Setup & Package Rename Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Set up full GitHub infrastructure (templates, workflows, tooling configs) and rename all packages from `sentinel-*` to `sntl-*` for crates.io publishing.

**Architecture:** Rename first (everything depends on correct package names), then layer in config files, templates, and workflows. cargo-husky replaces manual .githooks/.

**Tech Stack:** GitHub Actions, release-please, cargo-deny, cargo-husky, cargo-llvm-cov, bacon

---

### Task 1: Rename packages sentinel-* → sntl-*

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `sentinel-core/Cargo.toml` → rename dir to `sntl-core/Cargo.toml`
- Modify: `sentinel-macros/Cargo.toml` → rename dir to `sntl-macros/Cargo.toml`
- Modify: `sentinel-migrate/Cargo.toml` → rename dir to `sntl-migrate/Cargo.toml`
- Modify: `sentinel-cli/Cargo.toml` → rename dir to `sntl-cli/Cargo.toml`
- Modify: all `*.rs` files with `sentinel_core`, `sentinel_macros`, `sentinel_migrate` references
- Modify: `.github/workflows/codecov.yml`
- Modify: `CLAUDE.md`

**Step 1: Rename directories**

```bash
git mv sentinel-core sntl-core
git mv sentinel-macros sntl-macros
git mv sentinel-migrate sntl-migrate
git mv sentinel-cli sntl-cli
```

**Step 2: Update workspace root Cargo.toml**

```toml
[workspace]
members = [
    "sntl-core",
    "sntl-macros",
    "sntl-migrate",
    "sntl-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
repository = "https://github.com/cntm-labs/sentinel"
rust-version = "1.85"

[workspace.dependencies]
sntl-core = { path = "sntl-core" }
sntl-macros = { path = "sntl-macros" }
sntl-migrate = { path = "sntl-migrate" }
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
syn = { version = "2", features = ["full", "extra-traits"] }
quote = "1"
darling = "0.20"
proc-macro2 = "1"
proc-macro-error2 = "2"
trybuild = "1"
sentinel-driver = { git = "https://github.com/cntm-labs/sentinel-driver.git", tag = "sentinel-driver-v0.1.0" }
bytes = "1"
```

**Step 3: Update each crate's Cargo.toml**

`sntl-macros/Cargo.toml`:
```toml
[package]
name = "sntl-macros"
version.workspace = true
edition.workspace = true

[lib]
proc-macro = true

[dependencies]
syn.workspace = true
quote.workspace = true
darling.workspace = true
proc-macro2.workspace = true
proc-macro-error2.workspace = true

[dev-dependencies]
sntl-core.workspace = true
trybuild.workspace = true
uuid.workspace = true
chrono.workspace = true
sentinel-driver.workspace = true
```

`sntl-core/Cargo.toml`:
```toml
[package]
name = "sntl-core"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror.workspace = true
chrono.workspace = true
uuid.workspace = true
tokio.workspace = true
sntl-macros.workspace = true
sentinel-driver.workspace = true
bytes.workspace = true
```

`sntl-migrate/Cargo.toml`:
```toml
[package]
name = "sntl-migrate"
version.workspace = true
edition.workspace = true

[dependencies]
sntl-core.workspace = true
```

`sntl-cli/Cargo.toml`:
```toml
[package]
name = "sntl-cli"
version.workspace = true
edition.workspace = true

[dependencies]
sntl-core.workspace = true
sntl-migrate.workspace = true
tokio.workspace = true
```

**Step 4: Replace all Rust source references**

In all `.rs` files under `sntl-core/`, `sntl-macros/`, `sntl-migrate/`, `sntl-cli/`:

- `sentinel_core` → `sntl_core` (underscore form, used in `use` statements and paths)
- `sentinel_macros` → `sntl_macros` (underscore form)
- `sentinel_migrate` → `sntl_migrate` (underscore form)

In macro codegen files (`sntl-macros/src/model/codegen.rs`, `sntl-macros/src/partial/codegen.rs`):
- `sentinel_core::` → `sntl_core::` in all `quote!` blocks

**Step 5: Update codecov.yml ignore regex**

```yaml
--ignore-filename-regex '(sntl-cli/|sntl-macros/|query/exec\.rs)'
```

**Step 6: Update CLAUDE.md workspace structure**

Replace directory names in the structure diagram.

**Step 7: Verify**

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

**Step 8: Commit**

```bash
git add -A
git commit -m "refactor: rename packages sentinel-* → sntl-* for crates.io"
```

---

### Task 2: Dev tooling config files

**Files:**
- Create: `.editorconfig`
- Create: `.gitattributes`
- Create: `rustfmt.toml`
- Create: `clippy.toml`
- Create: `rust-toolchain.toml`
- Create: `bacon.toml`
- Create: `deny.toml`
- Modify: `.gitignore` (add coverage, IDE, OS entries)

**Step 1: Create `.editorconfig`**

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 4

[*.yml]
indent_size = 2

[*.md]
trim_trailing_whitespace = false
```

**Step 2: Create `.gitattributes`**

```
* text=auto eol=lf
```

**Step 3: Create `rustfmt.toml`**

```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
```

**Step 4: Create `clippy.toml`**

```toml
cognitive-complexity-threshold = 30
too-many-lines-threshold = 150
too-large-for-stack = 256
```

**Step 5: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
```

**Step 6: Create `bacon.toml`**

```toml
default_job = "check"

[jobs.check]
command = ["cargo", "check", "--workspace", "--color", "always"]
watch = ["sntl-core", "sntl-macros", "sntl-migrate", "sntl-cli"]

[jobs.clippy]
command = ["cargo", "clippy", "--workspace", "--all-targets", "--color", "always", "--", "-D", "warnings"]
watch = ["sntl-core", "sntl-macros", "sntl-migrate", "sntl-cli"]

[jobs.test]
command = ["cargo", "test", "--workspace", "--color", "always"]
watch = ["sntl-core", "sntl-macros", "sntl-migrate", "sntl-cli"]

[jobs.fmt]
command = ["cargo", "fmt", "--all"]
watch = ["sntl-core", "sntl-macros", "sntl-migrate", "sntl-cli"]

[keybindings]
c = "job:clippy"
t = "job:test"
f = "job:fmt"
```

**Step 7: Create `deny.toml`**

```toml
[graph]
all-features = true

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]

[licenses]
private = { ignore = true }
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "BSL-1.0",
    "0BSD",
    "Zlib",
]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "simplest-path"

[sources]
unknown-registry = "deny"
unknown-git = "allow"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = ["https://github.com/cntm-labs/sentinel-driver.git"]
```

**Step 8: Update `.gitignore`**

```gitignore
# Build
/target

# Lock (workspace)
Cargo.lock

# Worktrees
.worktrees/

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Coverage
coverage-html/
lcov.info
*.profraw

# Environment
.env
```

**Step 9: Verify**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

**Step 10: Commit**

```bash
git add .editorconfig .gitattributes rustfmt.toml clippy.toml rust-toolchain.toml bacon.toml deny.toml .gitignore
git commit -m "chore: add dev tooling configs (editorconfig, rustfmt, clippy, bacon, deny)"
```

---

### Task 3: Community files + cargo-husky

**Files:**
- Create: `CONTRIBUTING.md`
- Create: `SECURITY.md`
- Modify: workspace `Cargo.toml` (add cargo-husky dev-dep)

**Step 1: Create `CONTRIBUTING.md`**

```markdown
# Contributing to Sentinel

## Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Run checks:
   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```
5. Commit and open a pull request

## Conventions

See [CLAUDE.md](CLAUDE.md) for project conventions, lint policy, and architecture.

Key rules:
- Zero `unsafe` in sntl-core
- All queries parameterized at every layer (no SQL injection possible)
- Every model field should have `doc = "..."` attribute
- Migrations are plain SQL files
- 100% test coverage target for sntl-core

## Pre-commit Hook

Pre-commit hooks are managed by `cargo-husky` and install automatically on first `cargo test`.

The hook runs:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace --quiet`

## Workspace Structure

```
sentinel/
├── sntl-core/       # Model trait, QueryBuilder, Transaction, Relations
├── sntl-macros/     # derive(Model), derive(Partial), #[reducer]
├── sntl-migrate/    # Schema diff, migration generation
├── sntl-cli/        # CLI binary (`sentinel` command)
├── examples/        # Usage examples
└── docs/            # Design and implementation plans
```
```

**Step 2: Create `SECURITY.md`**

```markdown
# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do not** open a public GitHub issue
2. Use [GitHub Security Advisories](https://github.com/cntm-labs/sentinel/security/advisories/new) to report privately
3. Or email: security@cntm-labs.dev

We will acknowledge receipt within 48 hours and aim to provide a fix within 7 days for critical issues.

## Scope

This covers vulnerabilities in:
- SQL injection through the ORM query layer
- Migration safety (destructive operations, data loss)
- Connection credential handling
- Macro-generated code safety
- Type coercion exploits
```

**Step 3: Add cargo-husky to workspace dev-dependencies**

Add to workspace `Cargo.toml`:

```toml
[workspace.dependencies]
# ... existing deps ...
cargo-husky = { version = "1", default-features = false, features = ["precommit-hook", "run-cargo-fmt", "run-cargo-clippy", "run-cargo-test"] }
```

Add to `sntl-core/Cargo.toml` (only one crate needs it):

```toml
[dev-dependencies]
cargo-husky.workspace = true
```

**Step 4: Verify hooks install**

```bash
cargo test -p sntl-core --lib 2>&1 | head -5
ls -la .git/hooks/pre-commit
```

**Step 5: Commit**

```bash
git add CONTRIBUTING.md SECURITY.md Cargo.toml sntl-core/Cargo.toml
git commit -m "chore: add CONTRIBUTING.md, SECURITY.md, and cargo-husky pre-commit hooks"
```

---

### Task 4: GitHub issue & PR templates

**Files:**
- Create: `.github/pull_request_template.md`
- Create: `.github/ISSUE_TEMPLATE/bug_report.yml`
- Create: `.github/ISSUE_TEMPLATE/feature_request.yml`
- Create: `.github/ISSUE_TEMPLATE/documentation.yml`
- Create: `.github/ISSUE_TEMPLATE/config.yml`

**Step 1: Create `.github/pull_request_template.md`**

```markdown
## Summary

<!-- Brief description of the changes -->

## Changes

-

## Test Plan

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] New tests added for new functionality

## Checklist

- [ ] Code follows project conventions (see CLAUDE.md)
- [ ] Zero `unsafe` in sntl-core
- [ ] All queries parameterized
- [ ] Model fields have `doc = "..."` attribute
- [ ] Public APIs are documented
```

**Step 2: Create `.github/ISSUE_TEMPLATE/bug_report.yml`**

```yaml
name: Bug Report
description: Report a bug in Sentinel ORM
labels: ["bug"]
body:
  - type: dropdown
    id: area
    attributes:
      label: Area
      description: Which part of the ORM is affected?
      options:
        - Core (Model trait, QueryBuilder, types)
        - Macros (derive(Model), derive(Partial))
        - Migrations (schema diff, SQL generation)
        - CLI (sentinel command)
        - Driver Integration (connection, pool, execution)
        - Other
    validations:
      required: true
  - type: dropdown
    id: priority
    attributes:
      label: Severity
      description: How severe is this bug?
      options:
        - High — crash, data loss, or security issue
        - Medium — incorrect behavior with workaround
        - Low — minor or cosmetic issue
    validations:
      required: true
  - type: textarea
    id: description
    attributes:
      label: Description
      description: A clear description of the bug.
    validations:
      required: true
  - type: textarea
    id: steps
    attributes:
      label: Steps to Reproduce
      description: Minimal code or steps to reproduce the issue.
      placeholder: |
        1. Define model with...
        2. Build query...
        3. Observe error...
  - type: textarea
    id: expected
    attributes:
      label: Expected Behavior
  - type: textarea
    id: actual
    attributes:
      label: Actual Behavior
  - type: input
    id: rust-version
    attributes:
      label: Rust Version
      placeholder: "e.g., 1.85.0"
  - type: input
    id: pg-version
    attributes:
      label: PostgreSQL Version
      placeholder: "e.g., 16.2"
  - type: input
    id: os
    attributes:
      label: Operating System
      placeholder: "e.g., Ubuntu 24.04, macOS 15"
```

**Step 3: Create `.github/ISSUE_TEMPLATE/feature_request.yml`**

```yaml
name: Feature Request
description: Suggest a new feature or improvement
labels: ["enhancement"]
body:
  - type: dropdown
    id: area
    attributes:
      label: Area
      description: Which part of the ORM does this relate to?
      options:
        - Core (Model trait, QueryBuilder, types)
        - Macros (derive(Model), derive(Partial))
        - Migrations (schema diff, SQL generation)
        - CLI (sentinel command)
        - Driver Integration (connection, pool, execution)
        - Other
    validations:
      required: true
  - type: dropdown
    id: priority
    attributes:
      label: Priority
      description: How important is this feature?
      options:
        - High — blocks production use
        - Medium — important but has workaround
        - Low — nice to have
    validations:
      required: true
  - type: textarea
    id: description
    attributes:
      label: Description
      description: What feature would you like to see?
    validations:
      required: true
  - type: textarea
    id: use-case
    attributes:
      label: Use Case
      description: Describe the problem this would solve.
  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives Considered
      description: Any workarounds or alternative approaches you've considered.
```

**Step 4: Create `.github/ISSUE_TEMPLATE/documentation.yml`**

```yaml
name: Documentation
description: Report a documentation issue or suggest an improvement
labels: ["documentation"]
body:
  - type: textarea
    id: description
    attributes:
      label: Description
      description: What documentation is missing, incorrect, or could be improved?
    validations:
      required: true
  - type: input
    id: location
    attributes:
      label: Location
      description: Where is the issue? (URL, file path, or section name)
```

**Step 5: Create `.github/ISSUE_TEMPLATE/config.yml`**

```yaml
blank_issues_enabled: false
contact_links:
  - name: Question
    url: https://github.com/cntm-labs/sentinel/discussions
    about: Ask questions and discuss ideas
```

**Step 6: Commit**

```bash
git add .github/pull_request_template.md .github/ISSUE_TEMPLATE/
git commit -m "chore: add GitHub issue and PR templates"
```

---

### Task 5: Update CI workflow + codecov

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/codecov.yml`
- Create: `codecov.yml` (root — Codecov app config)

**Step 1: Update `ci.yml` — add rust-toolchain.toml auto-detection**

The existing ci.yml is already well-structured with parallel jobs. Update to remove explicit component declarations (rust-toolchain.toml handles this) and add `cargo deny` step:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -Dwarnings

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace

  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  deny:
    name: Dependency Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
```

**Step 2: Update `.github/workflows/codecov.yml`**

Update ignore regex for renamed packages and bump codecov-action to v5:

```yaml
name: Coverage

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  coverage:
    name: Coverage (100% target)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov
      - name: Generate coverage
        run: >-
          cargo llvm-cov --workspace
          --ignore-filename-regex '(sntl-cli/|sntl-macros/|query/exec\.rs)'
          --lcov --output-path lcov.info
          --fail-under-lines 100
      - name: Upload to Codecov
        uses: codecov/codecov-action@v5
        with:
          files: lcov.info
          fail_ci_if_error: true
        env:
          CODECOV_TOKEN: ${{ secrets.CODECOV_TOKEN }}
```

**Step 3: Create `codecov.yml` (root — Codecov app config)**

```yaml
coverage:
  status:
    project:
      default:
        target: auto
        threshold: 1%
    patch:
      default:
        target: 100%

ignore:
  - "sntl-cli/"
  - "sntl-macros/"
  - "sntl-core/src/query/exec.rs"

comment:
  layout: "reach,diff,flags,files"
  behavior: default
  require_changes: true
```

**Step 4: Verify**

```bash
cargo check --workspace
cargo test --workspace
```

**Step 5: Commit**

```bash
git add .github/workflows/ci.yml .github/workflows/codecov.yml codecov.yml
git commit -m "chore: update CI with cargo-deny, bump codecov to v5, add codecov.yml config"
```

---

### Task 6: New workflows — security, labeler, PR automation

**Files:**
- Create: `.github/workflows/security.yml`
- Create: `.github/workflows/labeler.yml`
- Create: `.github/workflows/pr-automation.yml`
- Create: `.github/labeler.yml`

**Step 1: Create `.github/workflows/security.yml`**

```yaml
name: Security

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  cargo-audit:
    name: Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-audit --locked
      - run: cargo audit

  cargo-deny:
    name: Deny
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
```

**Step 2: Create `.github/labeler.yml`**

```yaml
core:
  - changed-files:
      - any-glob-to-any-file: 'sntl-core/**'

macros:
  - changed-files:
      - any-glob-to-any-file: 'sntl-macros/**'

migrate:
  - changed-files:
      - any-glob-to-any-file: 'sntl-migrate/**'

cli:
  - changed-files:
      - any-glob-to-any-file: 'sntl-cli/**'

documentation:
  - changed-files:
      - any-glob-to-any-file: ['docs/**', '*.md']

ci:
  - changed-files:
      - any-glob-to-any-file: ['.github/**']
```

**Step 3: Create `.github/workflows/labeler.yml`**

```yaml
name: Labeler

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  label:
    name: Auto-label
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
    steps:
      - uses: actions/labeler@v5
        with:
          configuration-path: .github/labeler.yml
```

**Step 4: Create `.github/workflows/pr-automation.yml`**

```yaml
name: PR Automation

on:
  pull_request:
    types: [opened]

jobs:
  assign:
    name: Auto-assign
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
    steps:
      - name: Assign PR author
        uses: actions/github-script@v7
        with:
          script: |
            const login = context.payload.pull_request.user.login;
            await github.rest.issues.addAssignees({
              owner: context.repo.owner,
              repo: context.repo.repo,
              issue_number: context.payload.pull_request.number,
              assignees: [login],
            });
```

**Step 5: Commit**

```bash
git add .github/workflows/security.yml .github/workflows/labeler.yml .github/workflows/pr-automation.yml .github/labeler.yml
git commit -m "chore: add security audit, auto-labeler, and PR auto-assign workflows"
```

---

### Task 7: New workflows — pr-issue-link, weekly-digest, claude

**Files:**
- Create: `.github/workflows/pr-issue-link.yml`
- Create: `.github/workflows/weekly-digest.yml`
- Create: `.github/workflows/claude.yml`

**Step 1: Create `.github/workflows/pr-issue-link.yml`**

```yaml
name: Link Issue

on:
  pull_request:
    types: [opened]

jobs:
  link-issue:
    name: Auto-link issue
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
    steps:
      - name: Extract issue number from branch and link
        uses: actions/github-script@v7
        with:
          script: |
            const branch = context.payload.pull_request.head.ref;
            const match = branch.match(/(\d+)/);
            if (!match) {
              console.log('No issue number found in branch name');
              return;
            }
            const issueNumber = parseInt(match[1], 10);

            // Verify issue exists
            try {
              await github.rest.issues.get({
                owner: context.repo.owner,
                repo: context.repo.repo,
                issue_number: issueNumber,
              });
            } catch {
              console.log(`Issue #${issueNumber} not found, skipping`);
              return;
            }

            // Add "Closes #N" to PR body
            const pr = context.payload.pull_request;
            const currentBody = pr.body || '';
            if (currentBody.includes(`#${issueNumber}`)) {
              console.log(`PR body already references #${issueNumber}`);
              return;
            }

            const newBody = currentBody + `\n\nCloses #${issueNumber}`;
            await github.rest.pulls.update({
              owner: context.repo.owner,
              repo: context.repo.repo,
              pull_number: pr.number,
              body: newBody,
            });
            console.log(`Linked PR to issue #${issueNumber}`);
```

**Step 2: Create `.github/workflows/weekly-digest.yml`**

```yaml
name: Weekly Digest

on:
  schedule:
    - cron: '0 9 * * 1'

jobs:
  digest:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      issues: write
      id-token: write
    steps:
      - uses: actions/checkout@v4
      - uses: anthropics/claude-code-action@v1
        with:
          claude_code_oauth_token: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
          direct_prompt: |
            Generate a weekly engineering digest for Sentinel ORM.
            Analyze the repository and produce a summary covering:

            1. **Test Suite Status** — run `cargo test --workspace` and report results
            2. **Coverage Trends** — check recent coverage reports
            3. **New Dependencies** — compare Cargo.lock changes from last week
            4. **Open Issues Summary** — list open issues grouped by label
            5. **Recent PRs** — summarize merged and open PRs from the past week
            6. **Security Advisories** — run `cargo audit` and report findings
            7. **Action Items** — suggest priorities for next week

            Format as a GitHub issue titled "Weekly Digest — YYYY-MM-DD".
            Create the issue using `gh issue create`.
```

**Step 3: Create `.github/workflows/claude.yml`**

```yaml
name: Claude Code

on:
  pull_request:
    types: [opened, synchronize]
  issue_comment:
    types: [created]

jobs:
  claude:
    if: |
      (github.event_name == 'pull_request') ||
      (github.event_name == 'issue_comment' && contains(github.event.comment.body, '@claude'))
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
      issues: write
      id-token: write
    steps:
      - name: Run Claude Code
        uses: anthropics/claude-code-action@v1
        with:
          anthropic_api_key: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
          allowed_tools: "Bash,Read,Glob,Grep,Edit,Write"
          direct_prompt: |
            You are a PR automation assistant for Sentinel ORM (a compile-time guarded Rust ORM for PostgreSQL).

            On PR open, do the following:

            1. **Assign** the PR author using: gh pr edit $PR_NUMBER --add-assignee "$PR_AUTHOR"

            2. **Label** the PR based on changed files and content. Available labels:
               - `enhancement` — new features or improvements
               - `bug` — bug fixes
               - `documentation` — docs or markdown changes only
               - `core` — changes to sntl-core/
               - `macros` — changes to sntl-macros/
               - `migrate` — changes to sntl-migrate/
               - `cli` — changes to sntl-cli/
               Apply all labels that match. Use: gh pr edit $PR_NUMBER --add-label "label1,label2"

            3. **Link issues** — look at the branch name and PR body for issue references (#N).
               If found and not already in the body, add "Closes #N" by updating the PR body.

            4. **Comment** on linked issues with a short note like "Implementation PR: #PR_NUMBER"

            Use `gh` CLI for all GitHub operations. The PR number is available in the environment.
        env:
          PR_NUMBER: ${{ github.event.pull_request.number || github.event.issue.number }}
          PR_AUTHOR: ${{ github.event.pull_request.user.login || github.event.issue.user.login }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Step 4: Commit**

```bash
git add .github/workflows/pr-issue-link.yml .github/workflows/weekly-digest.yml .github/workflows/claude.yml
git commit -m "chore: add pr-issue-link, weekly-digest, and Claude Code workflows"
```

---

### Task 8: Release & publish config

**Files:**
- Create: `release-please-config.json`
- Create: `.release-please-manifest.json`
- Create: `.github/workflows/release-please.yml`
- Create: `.github/workflows/publish-crates.yml`

**Step 1: Create `release-please-config.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "packages": {
    "sntl-macros": {
      "release-type": "rust",
      "component": "sntl-macros",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true
    },
    "sntl-core": {
      "release-type": "rust",
      "component": "sntl-core",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true,
      "extra-files": [
        {
          "type": "toml",
          "path": "Cargo.toml",
          "glob": true,
          "jsonpath": "$.dependencies.sntl-macros.version"
        }
      ]
    },
    "sntl-migrate": {
      "release-type": "rust",
      "component": "sntl-migrate",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true,
      "extra-files": [
        {
          "type": "toml",
          "path": "Cargo.toml",
          "glob": true,
          "jsonpath": "$.dependencies.sntl-core.version"
        }
      ]
    },
    "sntl-cli": {
      "release-type": "rust",
      "component": "sntl-cli",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true,
      "extra-files": [
        {
          "type": "toml",
          "path": "Cargo.toml",
          "glob": true,
          "jsonpath": "$.dependencies.sntl-core.version"
        }
      ]
    }
  },
  "group-pull-request-title-pattern": "chore: release ${version}",
  "linked-versions": [
    {
      "tag": "v",
      "components": ["sntl-macros", "sntl-core", "sntl-migrate", "sntl-cli"]
    }
  ]
}
```

**Step 2: Create `.release-please-manifest.json`**

```json
{
  "sntl-macros": "0.1.0",
  "sntl-core": "0.1.0",
  "sntl-migrate": "0.1.0",
  "sntl-cli": "0.1.0"
}
```

**Step 3: Create `.github/workflows/release-please.yml`**

```yaml
name: Release Please

on:
  push:
    branches: [main]

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      releases_created: ${{ steps.release.outputs.releases_created }}
      sntl-macros--release_created: ${{ steps.release.outputs['sntl-macros--release_created'] }}
      sntl-core--release_created: ${{ steps.release.outputs['sntl-core--release_created'] }}
      sntl-migrate--release_created: ${{ steps.release.outputs['sntl-migrate--release_created'] }}
      sntl-cli--release_created: ${{ steps.release.outputs['sntl-cli--release_created'] }}
      tag_name: ${{ steps.release.outputs['sntl-core--tag_name'] }}
    steps:
      - uses: googleapis/release-please-action@v4
        id: release
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json

  ci-gate:
    needs: release-please
    if: needs.release-please.outputs.releases_created == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo test --workspace

  publish-crates:
    needs: [release-please, ci-gate]
    if: needs.release-please.outputs.releases_created == 'true'
    runs-on: ubuntu-latest
    environment: crates-io
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Publish sntl-macros
        if: needs.release-please.outputs['sntl-macros--release_created'] == 'true'
        run: cargo publish -p sntl-macros --no-verify
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Wait for crates.io index
        if: needs.release-please.outputs['sntl-macros--release_created'] == 'true'
        run: sleep 30

      - name: Publish sntl-core
        if: needs.release-please.outputs['sntl-core--release_created'] == 'true'
        run: cargo publish -p sntl-core --no-verify
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Wait for crates.io index
        if: needs.release-please.outputs['sntl-core--release_created'] == 'true'
        run: sleep 30

      - name: Publish sntl-migrate
        if: needs.release-please.outputs['sntl-migrate--release_created'] == 'true'
        run: cargo publish -p sntl-migrate --no-verify
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Wait for crates.io index
        if: needs.release-please.outputs['sntl-migrate--release_created'] == 'true'
        run: sleep 30

      - name: Publish sntl-cli
        if: needs.release-please.outputs['sntl-cli--release_created'] == 'true'
        run: cargo publish -p sntl-cli --no-verify
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

**Step 4: Create `.github/workflows/publish-crates.yml`**

```yaml
name: Publish Crates (Manual)

on:
  workflow_dispatch:
    inputs:
      dry-run:
        description: "Dry run (no actual publish)"
        required: false
        default: "false"
        type: choice
        options:
          - "false"
          - "true"

jobs:
  publish:
    runs-on: ubuntu-latest
    environment: crates-io
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Verify tests pass
        run: cargo test --workspace

      - name: Publish sntl-macros
        run: cargo publish -p sntl-macros --no-verify ${{ inputs.dry-run == 'true' && '--dry-run' || '' }}
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Wait for crates.io index
        if: inputs.dry-run == 'false'
        run: sleep 30

      - name: Publish sntl-core
        run: cargo publish -p sntl-core --no-verify ${{ inputs.dry-run == 'true' && '--dry-run' || '' }}
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Wait for crates.io index
        if: inputs.dry-run == 'false'
        run: sleep 30

      - name: Publish sntl-migrate
        run: cargo publish -p sntl-migrate --no-verify ${{ inputs.dry-run == 'true' && '--dry-run' || '' }}
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Wait for crates.io index
        if: inputs.dry-run == 'false'
        run: sleep 30

      - name: Publish sntl-cli
        run: cargo publish -p sntl-cli --no-verify ${{ inputs.dry-run == 'true' && '--dry-run' || '' }}
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

**Step 5: Commit**

```bash
git add release-please-config.json .release-please-manifest.json .github/workflows/release-please.yml .github/workflows/publish-crates.yml
git commit -m "chore: add release-please and manual publish workflows"
```

---

### Task 9: Full verification

**Step 1: Run full check suite**

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

**Step 2: Verify all new files exist**

```bash
ls .editorconfig .gitattributes rustfmt.toml clippy.toml rust-toolchain.toml bacon.toml deny.toml
ls CONTRIBUTING.md SECURITY.md
ls codecov.yml release-please-config.json .release-please-manifest.json
ls .github/pull_request_template.md
ls .github/ISSUE_TEMPLATE/bug_report.yml .github/ISSUE_TEMPLATE/feature_request.yml .github/ISSUE_TEMPLATE/documentation.yml .github/ISSUE_TEMPLATE/config.yml
ls .github/labeler.yml
ls .github/workflows/ci.yml .github/workflows/codecov.yml .github/workflows/security.yml .github/workflows/labeler.yml .github/workflows/pr-automation.yml .github/workflows/pr-issue-link.yml .github/workflows/weekly-digest.yml .github/workflows/claude.yml .github/workflows/release-please.yml .github/workflows/publish-crates.yml
```

**Step 3: Verify no old sentinel-* references remain**

```bash
grep -r "sentinel_core\|sentinel_macros\|sentinel_migrate\|sentinel-core\|sentinel-macros\|sentinel-migrate\|sentinel-cli" --include="*.rs" --include="*.toml" --include="*.yml" | grep -v target | grep -v sentinel-driver | grep -v sentinel-orm | grep -v "cntm-labs/sentinel"
```

Expected: no output (all references renamed to sntl-*)

**Step 4: Fix any remaining issues**

If grep finds leftover references, fix them.

**Step 5: Final commit (if needed)**

```bash
git add -A
git commit -m "chore: final cleanup — verify all sentinel-* → sntl-* renamed"
```
