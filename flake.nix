{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        with pkgs;
        {
          packages = rec {
            samba-share = pkgs.callPackage ./package.nix {};
            default = samba-share;
          };
          devShells.default = mkShell {
            buildInputs = with pkgs; [
              rustc
              cargo
              rust-analyzer
              clippy
              rustfmt
              blueprint-compiler
              meson
              ninja
              libadwaita
              adwaita-icon-theme
              gtk4
              librsvg
              pkg-config
              glib
              gobject-introspection
              gsettings-desktop-schemas
              polkit
              parted
              e2fsprogs
              util-linux
            ];

            # Environment variables for development
            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

            # GSettings schemas
            XDG_DATA_DIRS = "${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk4}/share/gsettings-schemas/${pkgs.gtk4.name}:$XDG_DATA_DIRS";
          };
        }
      );
}
