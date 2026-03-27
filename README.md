# gitlet

`gitlet` gives you lightweight, local-only version control contexts inside any existing git repo. Files you add to a gitlet get their own independent commit history, are automatically excluded from the outer repo, and never get pushed to your team's remote. Your `.gitignore` stays clean.

**Use cases:** personal notes inside a project, local config overrides, secrets and credentials that must never leave your machine, AI context files like `CLAUDE.md`, `DESIGN.md`, or `TASKS.md` that you customize locally for your own workflow and want versioned without pushing to the shared repo.

---

## Install

```sh
cargo install gitlet
```

To build from source:

```sh
git clone https://github.com/deadsoftie/gitlet
cd gitlet
cargo build --release
# binary is at target/release/gitlet
```

Requires Rust 1.82 or later.

---

## Quick Start

```sh
# 1. Initialise a gitlet inside any existing git repo
gitlet init secrets

# 2. Track a file - it is immediately excluded from outer git
gitlet add .env.local --to secrets

# 3. Commit a snapshot
gitlet commit -m "add local db credentials" --to secrets

# 4. See what has changed
gitlet status secrets

# 5. Inspect history
gitlet log secrets
```

Working with multiple gitlets and the active default:

```sh
gitlet init notes
gitlet switch notes        # notes is now the active gitlet

gitlet add TODO.md         # --to is optional when targeting the active gitlet
gitlet commit -m "draft roadmap"

gitlet list
# * notes      (active)   1 file tracked
#   secrets               1 file tracked
```

---

## Using Gitlet With AI Coding Tools

If you use AI tools like Claude Code, Cursor, or Copilot, you likely have context files sitting in your repo - `CLAUDE.md`, `DESIGN.md`, `TASKS.md`, prompt files, or agent instructions. These files often need to be customized per developer: your local paths, your preferred style, your personal workflow tweaks. Committing them means either everyone gets your version or you are constantly dealing with merge conflicts. Gitignoring them means losing version history on files you actively iterate on.

Gitlet is a natural fit here. Track your AI context files in a personal gitlet, version every change you make to them, and keep them completely out of the shared repo.

The shared repo stays clean. And you get a full audit trail of how your AI context evolved over the course of a project - which turns out to be surprisingly useful when something stops working the way it used to.

---

## Command Reference

| Command                             | Description                                             |
| ----------------------------------- | ------------------------------------------------------- |
| `gitlet init [name]`                | Create a new gitlet (default name: `default`)           |
| `gitlet add <files>... [--to <n>]`  | Stage files in a gitlet and exclude them from outer git |
| `gitlet remove <file> [--to <n>]`   | Untrack a file and restore it to outer git visibility   |
| `gitlet commit -m <msg> [--to <n>]` | Commit staged changes in a gitlet                       |
| `gitlet status [name]`              | Show working-directory status for all gitlets or one    |
| `gitlet log [name]`                 | Show commit history for a gitlet                        |
| `gitlet list`                       | List all gitlets with file counts and active marker     |
| `gitlet switch <n>`                 | Change the active gitlet                                |

All commands that target a specific gitlet accept `--to <n>` to override the active gitlet without changing the global config.

---

## How It Works

On `gitlet init`, gitlet creates `.gitlet/<n>/` - a bare git repository managed via [libgit2](https://libgit2.org). It also adds `.gitlet/` to `.git/info/exclude` so the outer repo never sees the gitlet directory.

When you `gitlet add` a file, two things happen:

1. The file is staged in the target gitlet's bare repo index.
2. The file's path is appended to `.git/info/exclude` - the outer git now ignores it completely.

`gitlet remove` reverses both operations. Your project's `.gitignore` is never modified.

```
my-project/
├── .git/
│   └── info/
│       └── exclude        <- gitlet writes exclusions here, never .gitignore
├── .gitlet/
│   ├── config.toml        <- active gitlet + registry of all gitlets
│   └── secrets/           <- bare git repo: objects, HEAD, refs
├── .env.local             <- excluded from outer git, versioned by "secrets"
└── src/
```

Each gitlet is a fully valid bare git repository. Commits, blobs, and trees are stored in `.gitlet/<n>/objects/` using standard git object format.

---

## Limitations

- **Local only.** Gitlets are never pushed. There is no remote, clone, or collaboration support in v1.
- **No branching.** Each gitlet has a single linear history. Branch management is not yet supported.
- **No diff command.** Use `gitlet log` to inspect history; working-tree diffs are not yet exposed.
- **No destroy command.** To remove a gitlet manually: delete `.gitlet/<n>/`, remove its entry from `.gitlet/config.toml`, and clean its paths from `.git/info/exclude`.
- **One file, one gitlet.** A file can only belong to one gitlet at a time.

---

## Roadmap

- `gitlet push` - push a gitlet as a git bundle or bare remote for backup or selective sharing
- `gitlet branch` / `gitlet checkout` - branching within a gitlet
- `gitlet diff` - show working-tree diff against the last gitlet commit
- `gitlet destroy <n>` - safely remove a gitlet and clean up all its exclusions
- Shell completions for all commands and gitlet names
