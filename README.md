# declair-rs

**declair-rs** is a small Rust command-line tool that helps you quickly add packages to your NixOS or Home Manager configuration. If you want, it can also trigger a rebuild (`nixos-rebuild` or `home-manager switch`) right after editing your config.

---

## What it does

* Search packages with `nix search --json`.
* Let you pick one interactively.
* Insert the package into a `with pkgs; [ ... ]` block in your config file.
* Create a backup before writing anything.
* Optionally run a rebuild command.

---

## Requirements

* `nix` command (with `nix search` available)
* `nixos-rebuild` and/or `home-manager` if you want auto rebuilds

---

## Installation

This project is packaged as a Nix flake.
Make sure you have flakes enabled in Nix.

You can run **declair-rs** directly with:
```bash
nix run github:timasoft/declair-rs
```

If you want to have declair-rs always available in your $PATH:
```bash
nix profile install github:timasoft/declair-rs
```

If you manage your NixOS configuration with flakes, add declair-rs as an input in your flake.nix:
```nix
{
  inputs.declair-rs.url = "github:timasoft/declair-rs";

  outputs = { self, nixpkgs, declair-rs, ... }:
    {
      nixosConfigurations.my-hostname = nixpkgs.lib.nixosSystem {
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

## Config file

On first run a config file will be created in your system’s config directory (for example `~/.config/declair/config.toml`).

Example:

```toml
nix_path = "~/nixos"
auto_rebuild = true
home_manager = false
flake = true
```

* `nix_path`: file or folder with your Nix configs (supports `~`).
* `auto_rebuild`: run rebuild/switch automatically after adding a package.
* `home_manager`: use `home-manager switch` instead of `nixos-rebuild`.
* `flake`: add `--flake .` to the rebuild command.

---

## How package insertion works

* Finds a `with pkgs; [ ... ]` block.
* Makes a `.declair.bak` backup.
* Skips if the package is already there.
* Inserts into single-line or multi-line lists, keeping indentation.

If your config is formatted in a very unusual way, double‑check the backup.

---

## Example session

1. Run `declair-rs` and point it at your config (e.g. `~/nixos/configuration.nix`).
2. Type a package name to search (e.g. `neovim`).
3. Select from the list.
4. If auto rebuild is enabled, it’ll run the appropriate command.
