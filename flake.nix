{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-23.11";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { nixpkgs, fenix, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rustPlatform = import fenix { inherit system; };
        rust = rustPlatform.stable.toolchain;
        craneLib = crane.lib.${system}.overrideToolchain rust;
        buildInputs = [ pkgs.sqlite ];
        binary = craneLib.buildPackage {
          src = craneLib.path ./.;
          buildInputs = buildInputs;

          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
        };
      in
      {
        packages.default = binary;

        devShells.default = craneLib.devShell {
          inputsFrom = [ binary ];
          packages = with pkgs; [ sea-orm-cli cargo-expand ];

          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
        };
      });
}
