# sh-hist

A terminal UI for searching zsh history.

## Usage

Run `sh-hist` directly to browse history. The interface starts in editing mode:

- Type to search. `Up`/`Down` or `Ctrl-P`/`Ctrl-N` moves the selection.
- Press `Enter` to select and execute the highlighted command through the zsh widget below.
- Press `Tab` to select the highlighted command for editing without executing it.
- Press `Esc` to enter normal mode; use `e`, `a`, or `i` to resume editing and `q` or `Esc` to quit.

When using `--result-file <path>`, a selection is written to that path. The
process exits with status `10` for execute or `11` for edit, enabling shell
widgets to handle the selected command.

## Build

1. Clone this repo
```sh
git clone git@github.com:avik-ch/sh-hist.git
```

2. Build the binary
```sh
cargo build --release
```

The binary will be created at `target/release/sh-hist`.

## Install

```sh
cargo install --path path/to/repo
```

This installs `sh-hist` into Cargo's binary directory, which should be on your `PATH`.

## Add widget to shell configuration

Add the following to `~/.zshrc`. It binds `Ctrl-R` to history search.

```sh
sh-hist-widget() {
  emulate -L zsh

  local result_file exit_code selected
  result_file=$(mktemp "${TMPDIR:-/tmp}/sh-hist.XXXXXX") || return 1

  zle -I

  command sh-hist --result-file "$result_file" \
    </dev/tty >/dev/tty 2>/dev/tty
  exit_code=$?

  if (( exit_code == 10 || exit_code == 11 )) && [[ -f $result_file ]]; then
    selected=$(<"$result_file")
  fi

  rm -f -- "$result_file"

  if (( exit_code == 10 )); then
    BUFFER=$selected
    CURSOR=${#BUFFER}
    zle accept-line
  elif (( exit_code == 11 )); then
    BUFFER=$selected
    CURSOR=${#BUFFER}
    zle reset-prompt
  else
    zle reset-prompt
  fi

  return 0
}

zle -N sh-hist-widget
bindkey '^R' sh-hist-widget
```

Then, open a new shell or source your configuration.

This only works for `zsh`. It should be possible to get most of the functionality in `bash` by tweaking the widget.
