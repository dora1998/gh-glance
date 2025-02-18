# gh-glance

GitHub CLI extension for quickly checking (glancing at) PRs using worktrees.

## Motivation

When reviewing a pull request, you often need to test it locally. However, switching branches requires committing your local changes first.

[`git worktree`](https://git-scm.com/docs/git-worktree) solves this by letting you check out a branch in a separate directory while keeping your working branch untouched.

One caveat is that files listed in `.gitignore` (like `node_modules`) aren't present in the new worktree. For a Node.js project, you'd typically need to:

```shell
git worktree add ./.worktree/feature-branch origin/feature-branch
cd ./.worktree/feature-branch
npm install
npm run dev
```

`gh-glance` simplifies this process into a single command that handles all the necessary worktree setup:

```shell
gh glance run 1234 dev
```

## Installation

```shell
gh extension install dora1998/gh-glance
```

Make sure to add the worktree root directory (default: `.worktree/`) to your `.gitignore`:

```
.worktree/
```

## Usage

### `run`

Run defined tasks in the pr's worktree. You can also omit `run`.
You can specify either pr number or branch name.

```shell
# Pull request number
gh glance run 1234 storybook
# Branch name
gh glance run feature/foo storybook
# Omit `run`
gh glance 1234 storybook
```

Run any command with `--`.

```shell
gh glance 1234 -- pwd
```

### `dir`

Get the PR's worktree directory path. Useful with `cd` command:

```shell
cd $(gh glance dir 1234)
```

### `checkout`

Add the pr's worktree.

```shell
gh glance checkout 1234
```

### `remove`

Remove the pr's worktree.

```shell
gh glance rm 1234
```

### `clean`

Remove all merged branch's worktrees.

```shell
gh glance clean
```

## Configuration

Configuration should be written in `.gh-glance.toml` file placed in the project root directory.

Below is an example configuration:

```toml
[base]
worktree_dir = ".worktree/"
prepare_task = "prepare"
auto_checkout = false
auto_pull = "force"

[tasks.prepare]
run = "cp .env.sample .env.local && bun i"

[tasks.dev]
run = "bun dev"

[tasks.storybook]
run = "bun storybook"
```

### `base.worktree_dir`

- Type: String
- Default: ".worktree/"
- Description: Directory name for worktree root.

### `base.prepare_task`

- Type: String
- Default: "" (empty string)
- Description: Task name to run before each task. This task will be executed before running any other task, except when running the prepare task itself.

### `base.auto_checkout`

- Type: Boolean
- Default: true
- Description: Whether to automatically checkout the branch when running a task.

### `base.auto_pull`

- Type: String
- Values: "default" | "force" | "off"
- Default: "default"
- Description: Determines how to update the worktree when checking out
  - `default`: Performs a normal `git pull`
  - `force`: Performs `git fetch` followed by `git reset --hard origin/<branch>`
  - `off`: Skips any update operation

### `tasks.<name>.run`

- Type: String
- Description: Command to run for this task. The command will be executed in the worktree directory.

## Development

### Prerequisites

- Rust toolchain (1.70.0 or later)
- GitHub CLI
- GitHub Personal Access Token (with `repo` scope)

### Build

```shell
cargo build
```

### Run the project locally

```shell
cargo run -- checkout 1234 # Checkout PR #1234
cargo run -- run 1234 dev # Run dev task in PR #1234's worktree
```
