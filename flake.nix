{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    vscode-extensions.url = "github:nix-community/nix-vscode-extensions";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      vscode-extensions,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };
        vscode-marketplace = vscode-extensions.extensions.${system}.vscode-marketplace;
        vs = pkgs.vscode-with-extensions.override {
          vscodeExtensions =
            (with pkgs.vscode-extensions; [
              rust-lang.rust-analyzer
              jnoortheen.nix-ide
              wholroyd.jinja
            ])
            ++ [
              # marketplace.drblury.protobuf-vsc
            ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # Rust toolchain
            rustup
            pre-commit

            # Vscode extensions
            vs
          ];

          # Environment variables
          shellHook = ''
            export RUSTUP_HOME=$HOME/.rustup
            export CARGO_HOME=$HOME/.cargo
            export PATH="$RUSTUP_HOME/bin:$CARGO_HOME/bin:$PATH"

            if [ ! -d "$RUSTUP_HOME" ]; then
              rustup default stable
              rustup component add clippy rustfmt rust-analyzer
            fi
          '';
        };

        # Build the crate with `nix build`
        packages.default = pkgs.rustPlatform.buildRustPackage rec {
          pname = "landing";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = with pkgs; [
          ];
          doCheck = false;
        };
      }
    );
}
