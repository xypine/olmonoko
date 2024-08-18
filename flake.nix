{
  description = "Shell with rust dev env & docker";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, flake-utils, nixpkgs, rust-overlay }@inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            # Docker
            pkgs.docker
            pkgs.docker-compose

            # Rust
            pkgs.rust-bin.nightly.latest.default
            pkgs.cargo # Package manager
            pkgs.rustfmt # Formatting
            pkgs.bacon # Constant feedback
            pkgs.clippy # Tips & tricks
          ];
          shellHook = ''
            # Add shell commands here
          '';
        };
      });
}
