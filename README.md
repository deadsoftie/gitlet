# gitnook

`gitnook` gives you lightweight, local-only version control contexts inside any existing git repo. Files you add to a gitnook get their own independent commit history, are automatically excluded from the outer repo, and never get pushed to your team's remote. Your `.gitignore` stays clean.

**Use cases:** personal notes inside a project, local config overrides, secrets and credentials that must never leave your machine, AI context files like `CLAUDE.md`, `DESIGN.md`, or `TASKS.md` that you customize locally for your own workflow and want versioned without pushing to the shared repo.

---

## Install

```sh
cargo install gitnook
```

To build from source:

```sh
git clone https://github.com/deadsoftie/gitnook
cd gitnook
cargo build --release
# binary is at target/release/gitnook

# to install it globally so you can run gitnook from anywhere
cargo install --path .

# verify the install
gitnook --version
```

Requires Rust 1.82 or later.

---

## Quick Start

```sh
# 1. Initialise a gitnook inside any existing git repo
gitnook init secrets

# 2. Track a file - it is immediately excluded from outer git
gitnook add .env.local --to secrets

# 3. Commit a snapshot
gitnook commit -m "add local db credentials" --to secrets

# 4. See what has changed
gitnook status secrets

# 5. Inspect history
gitnook log secrets
```

Working with multiple gitnooks and the active default:

```sh
gitnook init notes
gitnook switch notes        # notes is now the active gitnook

gitnook add TODO.md         # --to is optional when targeting the active gitnook
gitnook commit -m "draft roadmap"

gitnook list
# * notes      (active)   1 file tracked
#   secrets               1 file tracked
```

---

## Using Gitnook With AI Coding Tools

If you use AI tools like Claude Code, Cursor, or Copilot, you likely have context files sitting in your repo - `CLAUDE.md`, `DESIGN.md`, `TASKS.md`, prompt files, or agent instructions. These files often need to be customized per developer: your local paths, your preferred style, your personal workflow tweaks. Committing them means either everyone gets your version or you are constantly dealing with merge conflicts. Gitignoring them means losing version history on files you actively iterate on.

Gitnook is a natural fit here. Track your AI context files in a personal gitnook, version every change you make to them, and keep them completely out of the shared repo.

The shared repo stays clean. And you get a full audit trail of how your AI context evolved over the course of a project - which turns out to be surprisingly useful when something stops working the way it used to.

---

## Command Reference

| Command                             | Description                                             |
| ----------------------------------- | ------------------------------------------------------- |
| `gitnook init [name]`                | Create a new gitnook (default name: `default`)           |
| `gitnook add <files>... [--to <n>]`  | Stage files in a gitnook and exclude them from outer git |
| `gitnook remove <file> [--to <n>]`   | Untrack a file and restore it to outer git visibility   |
| `gitnook commit -m <msg> [--to <n>]` | Commit staged changes in a gitnook                       |
| `gitnook status [name]`              | Show working-directory status for all gitnooks or one    |
| `gitnook log [name]`                 | Show commit history for a gitnook                        |
| `gitnook list`                       | List all gitnooks with file counts and active marker     |
| `gitnook switch <n>`                 | Change the active gitnook                                |
| `gitnook diff [name]`                | Show working-tree diff against the last gitnook commit  |

All commands that target a specific gitnook accept `--to <n>` to override the active gitnook without changing the global config.

---

## How It Works

On `gitnook init`, gitnook creates `.gitnook/<n>/` - a bare git repository managed via [libgit2](https://libgit2.org). It also adds `.gitnook/` to `.git/info/exclude` so the outer repo never sees the gitnook directory.

When you `gitnook add` a file, two things happen:

1. The file is staged in the target gitnook's bare repo index.
2. The file's path is appended to `.git/info/exclude` - the outer git now ignores it completely.

`gitnook remove` reverses both operations. Your project's `.gitignore` is never modified.

```
my-project/
├── .git/
│   └── info/
│       └── exclude        <- gitnook writes exclusions here, never .gitignore
├── .gitnook/
│   ├── config.toml        <- active gitnook + registry of all gitnooks
│   └── secrets/           <- bare git repo: objects, HEAD, refs
├── .env.local             <- excluded from outer git, versioned by "secrets"
└── src/
```

Each gitnook is a fully valid bare git repository. Commits, blobs, and trees are stored in `.gitnook/<n>/objects/` using standard git object format.

---

## Limitations

- **Local only.** Gitnooks are never pushed. There is no remote, clone, or collaboration support in v1.
- **No branching.** Each gitnook has a single linear history. Branch management is not yet supported.
- **No diff command.** Use `gitnook log` to inspect history; working-tree diffs are not yet exposed.
- **No destroy command.** To remove a gitnook manually: delete `.gitnook/<n>/`, remove its entry from `.gitnook/config.toml`, and clean its paths from `.git/info/exclude`.
- **One file, one gitnook.** A file can only belong to one gitnook at a time.

---

## Roadmap

- `gitnook push` - push a gitnook as a git bundle or bare remote for backup or selective sharing
- `gitnook branch` / `gitnook checkout` - branching within a gitnook
- `gitnook destroy <n>` - safely remove a gitnook and clean up all its exclusions
- Shell completions for all commands and gitnook names
