{
  description = "The Greatest Wayland Compositor EVER";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        inherit (nixpkgs) lib;

        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
      in {

        devShells.default = pkgs.mkShell rec {
          packages = with pkgs; [
            rust-bin.nightly."2024-07-09".complete
            pkg-config
          ];

          buildInputs = with pkgs; [
              mesa
              wayland
              udev
              libinput
              libseat
              xwayland
              pixman
              libxkbcommon
          ];

          runtimeLibraries = with pkgs;
            with xorg; [
              libGL
              libX11
              libXcursor
              libxcb
              libXi
            ];

          LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibraries;
        };

        formatter = pkgs.nixfmt-classic;
      });
}
