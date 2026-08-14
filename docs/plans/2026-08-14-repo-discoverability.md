# Repository Discoverability and SEO Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make whytop easy to understand, find, install, and evaluate as a public Rust CLI repository, then publish the changes to a GitHub remote.

**Architecture:** Keep the application code unchanged and improve the repository surface: Cargo package metadata, README information architecture, standard GitHub community files, and a lightweight CI workflow. Use the existing project behavior as the source of truth and avoid claiming unsupported features.

**Tech Stack:** Rust/Cargo, Markdown, GitHub Actions, GitHub repository metadata via `gh`.

---

### Task 1: Add repository and package metadata

**Files:**
- Modify: `Cargo.toml`
- Create: `LICENSE`
- Create: `CONTRIBUTING.md`
- Create: `SECURITY.md`

**Step 1: Update Cargo metadata**

Add homepage, repository, readme, keywords, and categories that accurately describe a cross-platform process monitor and local AI tooling.

**Step 2: Add standard repository policy files**

Add the existing MIT license text, a focused contribution guide with local verification commands, and a security policy that explains the local-only data boundary and responsible disclosure path.

**Step 3: Verify metadata**

Run `cargo metadata --no-deps --format-version 1` and inspect that the package metadata parses successfully.

### Task 2: Rewrite the README for searchability and conversion

**Files:**
- Modify: `README.md`

**Step 1: Add a concise search-friendly project summary**

Describe whytop with natural-language terms users search for: terminal process monitor, `top`/`htop` alternative, read-only inspection, and local AI explanations.

**Step 2: Add practical discovery sections**

Document features, platform support, prerequisites, quick start, controls, local model setup, privacy/safety, troubleshooting, development, and roadmap/limitations without inventing functionality.

**Step 3: Add stable navigation and badges**

Add links to the repository, issues, license, and CI, using the eventual GitHub owner/name rather than broken placeholders.

### Task 3: Add GitHub indexing and quality signals

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/ISSUE_TEMPLATE/bug_report.md`
- Create: `.github/ISSUE_TEMPLATE/feature_request.md`

**Step 1: Add CI**

Run formatting, tests, and `cargo check` on pushes and pull requests across the supported stable Rust toolchain.

**Step 2: Add issue templates**

Capture platform, version, reproduction steps, and model-runtime details for useful issue content.

**Step 3: Validate YAML and Markdown structure**

Review the files and run the local Rust checks before publishing.

### Task 4: Create or connect the remote repository and publish

**Files:**
- No source files; Git history and remote configuration only.

**Step 1: Inspect GitHub authentication and repository availability**

Use `gh auth status` and check whether the authenticated account already has a `whytop` repository.

**Step 2: Create the public repository if needed**

Create `whytop` with the MIT license, set the description and topics, and connect it as `origin`.

**Step 3: Commit and push intentionally**

Stage explicit files only, commit the discoverability work, and push `master` without force-pushing.

**Step 4: Verify the published repository**

Confirm the remote URL, branch tracking, repository description/topics, and clean local status.
