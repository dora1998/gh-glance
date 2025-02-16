# gh-glance

## Installation

T.B.D.

## Usage

### `run`

Run defined tasks in the pr's worktree. You can also omit `run`.

```shell
gh glance run 1234 storybook
gh glance 1234 storybook
```

Run any command with `--`.

```shell
gh glance 1234 -- pwd
```

### `checkout`

Move to the pr's worktree.

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

```toml
[base]
prepare_task = "prepare"
auto_pull = "force"

[tasks.prepare]
run = "cp .env.sample .env.local && bun i"

[tasks.dev]
run = "bun dev"

[tasks.storybook]
run = "bun storybook"
```
