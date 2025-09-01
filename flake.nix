{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    naersk.url  = "github:nix-community/naersk";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, naersk, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs { inherit system; };
      naerskLib = pkgs.callPackage naersk {};
      rustSrc = pkgs.rust.packages.stable.rustPlatform.rustLibSrc;

      declair-rs = naerskLib.buildPackage {
        src = ./.;
        buildInputs = with pkgs; [  ];
        nativeBuildInputs = [ pkgs.pkg-config ];
      };
    in {
      packages.default = declair-rs;

      defaultPackage = declair-rs;

      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          fish
          cargo rustc rustfmt clippy rust-analyzer
        ];
        nativeBuildInputs = [ pkgs.pkg-config ];

        shellHook = ''
          if [ -z "$FISH_VERSION" ] && [ -z "$NO_AUTO_FISH" ]; then
            exec ${pkgs.fish}/bin/fish
          fi
        '';

        env.RUST_SRC_PATH = "${rustSrc}";
      };
    });
}
