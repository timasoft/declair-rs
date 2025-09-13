# declair-rs

**declair-rs** is a Rust command-line utility that helps you quickly search, add, and manage packages in your NixOS or Home Manager configuration. It can also optionally trigger an automatic rebuild (`nixos-rebuild` or `home-manager switch`) after modifying your config.

---

## Features

* Search packages using `nix search --json` and pick a result interactively.
* Insert package into a `with pkgs; [ ... ]` block (single-line or multi-line).
* Remove packages from that block (via `--remove`).
* List packages currently present in a config file (`--list`).
* Create a simple TOML config on first run (`~/.config/declair/config.toml`).
* Dry-run mode to preview selected package without making changes (`--dry-run`).
* Support for adding packages as `programs.<name>.enable = true;` when available (`--program`).

---

## Requirements

* `nix` (with `nix search` available)
* `nixos-rebuild` and/or `home-manager` if you want automatic rebuilds

---

## Install

Run directly from the flake:

```bash
nix run github:timasoft/declair-rs
```

Install permanently to your user profile:

```bash
nix profile install github:timasoft/declair-rs
```

Add as an input to your flake (example):

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

## Usage

```bash
declair-rs [OPTIONS]
```

Common options:

* `-c, --config <FILE>` — path to config file or directory (overrides stored config)
* `-p, --package <NAME>` — package name or search query
* `--no-interactive` — run without prompts (fails if required info is missing)
* `--no-rebuild` — skip automatic rebuild even if enabled in config
* `-r, --remove` — remove package from the `with pkgs; [...]` block
* `-l, --list` — list packages currently present in the `with pkgs; [...]` block
* `-d, --dry-run` — perform a dry-run (only print selected package without modifying files)
* `--program` — use `programs.<package>.enable = true;` instead of adding pkg to `with pkgs; [...]` (if available)

### Example

Interactive add:

```bash
declair-rs
# then type a query like `neovim` and choose a result
```

Non-interactive (add exact name):

```bash
declair-rs --no-interactive -p neovim
```

List packages in a config:

```bash
declair-rs --config /etc/nixos/configuration.nix --list
```

Remove a package:

```bash
declair-rs -c ~/nixos -p somepkg -r
```

Dry-run to preview selection:

```bash
declair-rs -d firefox
```

Add package as program (when available):

```bash
declair-rs -p firefox
```

---

## Configuration

On first run, the tool writes a small TOML config under the platform config dir (typically `~/.config/declair/config.toml`).

Example `config.toml`:

```toml
nix_path = "~/nixos"
auto_rebuild = true
home_manager = false
flake = true
```

Options:

* `nix_path` — path to your Nix configuration file or directory (tilde `~` is expanded)
* `auto_rebuild` — whether to run a rebuild after modifying the file
* `home_manager` — use `home-manager switch` instead of `nixos-rebuild`
* `flake` — append `--flake .` to rebuild commands

---

## Development

Use the provided dev shell (flake):

```bash
nix develop
```

It includes Rust toolchain components: `cargo`, `rustc`, `rustfmt`, `clippy`, `rust-analyzer`, and `fish` shell by default.

Build and run with cargo:

```bash
cargo run --release
```

---

## TODO

* [x] Add support for removing packages (`--remove`).
* [x] Implement listing of currently configured packages (`--list`).
* [x] Add `--dry-run` option to preview changes without writing.
* [x] Support `programs.<name>.enable = true;` style package declarations.
* [ ] Support multiple configuration files in a single profile.
* [ ] Add autocomplete for package names.
* [ ] Add GIF demo to the README
