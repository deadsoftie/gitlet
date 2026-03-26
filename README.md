# gitlet

> Lightweight local version control contexts inside any Git repo.

Gitlet lets you track files privately inside an existing Git repository. Files you add to a gitlet get their own commit history and are automatically excluded from the outer repo - your team never sees them, they never get pushed, and your `.gitignore` stays clean.

---

## The Problem

You're working inside a project repo and you want to version some files locally - personal notes, a scratch config, a `.env` file. Your options today are awkward:

- Add them to `.gitignore` - they're excluded but unversioned
- Commit them to the repo - now they're everyone's problem
- Create a separate repo somewhere else - now you're context-switching to track files that live _here_

Gitlet is the missing option: version those files right where they live, privately, without touching the outer repo.

---

## How It Works

Running `gitlet init` creates a `.gitlet/` directory at your repo root (automatically excluded from the outer git via `.git/info/exclude`). Each named gitlet inside it is a lightweight bare git repo with its own object store and commit history.

When you run `gitlet add notes.md`, two things happen:

1. `notes.md` is staged in your active gitlet
2. `notes.md` is added to `.git/info/exclude` so the outer git ignores it permanently

Your project's `.gitignore` is never touched.

---

## Install

```bash
cargo install gitlet
```

> Gitlet is currently in development. The binary is not yet published to crates.io. To build from source:
>
> ```bash
> git clone https://github.com/your-username/gitlet
> cd gitlet
> cargo build --release
> # binary is at target/release/gitlet
> ```

---

## Quick Start

```bash
# Initialize a gitlet inside your existing repo
gitlet init

# Start tracking a file
gitlet add notes.md

# Commit it
gitlet commit -m "initial notes"

# Check status across all gitlets
gitlet status
```

---

## Multiple Gitlets

You can have more than one gitlet per repo, each tracking different files independently:

```bash
gitlet init secrets
gitlet init scratch

gitlet add .env.local --to secrets
gitlet add todo.md --to scratch

gitlet list
# * secrets   (active)   1 file tracked
#   scratch              1 file tracked
```

---

## Commands

| Command                                | Description                                          |
| -------------------------------------- | ---------------------------------------------------- |
| `gitlet init [name]`                   | Create a new gitlet (default name: `default`)        |
| `gitlet add <files> [--to <name>]`     | Track files in the active or named gitlet            |
| `gitlet remove <file> [--to <name>]`   | Untrack a file and return it to outer git visibility |
| `gitlet commit -m <msg> [--to <name>]` | Commit staged changes                                |
| `gitlet status [name]`                 | Show status across all gitlets, or a specific one    |
| `gitlet log [name]`                    | Show commit history for a gitlet                     |
| `gitlet list`                          | List all gitlets in the current repo                 |
| `gitlet switch <name>`                 | Change the active gitlet                             |

---

## Design Principles

- **Never touches `.gitignore`** - exclusions go to `.git/info/exclude`, which is local-only and never committed
- **Fully local by default** - nothing gets pushed anywhere unless you explicitly choose to
- **One file, one gitlet** - a file can only be tracked by one gitlet at a time, preventing conflicts
- **Zero outer repo pollution** - the outer git remains completely unaware of gitlet and its files

---

## Limitations

- **Local only.** Gitlets exist only on your machine. There is no push, pull, or remote support in v1.
- **No branching.** Each gitlet has a single `main` branch. Branching within a gitlet is not yet supported.
- **No diff.** There is no `gitlet diff` command in v1. Use `gitlet log` to inspect history.
- **No destroy.** Deleting a gitlet is not yet supported. Files remain excluded from the outer git until manually cleaned from `.git/info/exclude`.
- **One file, one gitlet.** A file cannot be shared between gitlets.

---

## Roadmap

- **Remote push** — push a gitlet as a git remote or submodule so it can be backed up or shared selectively
- **Branching** — create and switch branches within a gitlet
- **Diff** — `gitlet diff` to compare working tree against the last commit
- **Destroy** — `gitlet destroy <name>` to remove a gitlet and clean up its exclusions
- **Shell completions** — tab completion for all commands and gitlet names

---

## Built With

- [Rust](https://www.rust-lang.org/)
- [clap](https://docs.rs/clap) - CLI argument parsing
- [git2](https://docs.rs/git2) - Git operations via libgit2
