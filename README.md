# declair-rs

**declair-rs** is a Rust command-line utility that helps you quickly search, add, and manage packages in your NixOS or Home Manager configuration. It can also optionally trigger an automatic rebuild (`nixos-rebuild` or `home-manager switch`) after modifying your config.

---

## Features

* Search packages using `nix search --json`.
* Interactive selection of results.
* Automatically insert the package into a `with pkgs; [ ... ]` block.
* Creates a `.declair.bak` backup before writing.
* Works with single-line and multi-line lists while keeping indentation.
* Optional automatic rebuild of your system or home environment.

---

## Requirements

* `nix` (with `nix search` enabled)
* `nixos-rebuild` and/or `home-manager` (for rebuild support)

---

## Installation

This project is packaged as a Nix flake.

Run directly:

```bash
nix run github:timasoft/declair-rs
```

Install permanently:

```bash
nix profile install github:timasoft/declair-rs
```

Add as an input to your own flake:

```nix
{
  inputs.declair-rs.url = "github:timasoft/declair-rs";

  outputs = { self, nixpkgs, declair-rs, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        {
          environment.systemPackages = [
            declair-rs.packages.x86_64-linux.default
          ];
        }
      ];
    };
  };
}
```

---

## Configuration

On first run, a config file is created under your system config directory (e.g. `~/.config/declair/config.toml`).

Example:

```toml
nix_path = "~/nixos"
auto_rebuild = true
home_manager = false
flake = true
```

### Options:

* `nix_path`: file or directory with your Nix configuration (`~` is supported).
* `auto_rebuild`: automatically run a rebuild after inserting a package.
* `home_manager`: use `home-manager switch` instead of `nixos-rebuild`.
* `flake`: append `--flake .` to rebuild commands.

---

## Command-line usage

```bash
declair-rs [OPTIONS]
```

### Available options:

* `-c, --config <FILE>` – specify path to configuration file or directory
* `-p, --package <NAME>` – package to add (used as query in interactive mode)
* `--no-interactive` – disable prompts; fail if information is missing
* `--no-rebuild` – skip rebuild even if config enables it

---

## How insertion works

1. Finds a `with pkgs; [ ... ]` block.
2. Makes a `.declair.bak` backup.
3. Skips insertion if package already exists.
4. Handles single-line (`with pkgs; [ foo bar ]`) and multi-line blocks.

If your configuration is formatted in an unusual way, check the backup file.

---

## Example workflow

```bash
declair-rs
```

1. Select your configuration file on first run.
2. Type a package name to search (e.g. `neovim`).
3. Choose a package from the results.
4. The tool edits your config and (if enabled) runs a rebuild.

---

## Development

A dev shell is provided via flakes:

```bash
nix develop
```

It includes Rust toolchain components: `cargo`, `rustc`, `rustfmt`, `clippy`, `rust-analyzer`, and `fish` shell by default.

## TODO

* [ ] Add support for removing packages (`--remove`).
* [ ] Implement listing of currently configured packages (`--list`).
* [ ] Add `--dry-run` option to preview changes without writing.
* [ ] Support multiple configuration files in a single profile.
* [ ] Add autocomplete for package names.
