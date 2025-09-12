{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
  };

  outputs =
    inputs@{
      flake-parts,
      nixpkgs,
      self,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

      perSystem =
        { pkgs, ... }:
        let
          ascii-image-converter' = pkgs.ascii-image-converter.overrideAttrs (old: {
            postPatch = ''
              substituteInPlace aic_package/util.go \
                --replace-fail "saveAscii := flattenAscii(asciiSet, false, true)" \
                "saveAscii := flattenAscii(asciiSet, true, false)"
            '';
          });
        in
        {
          devShells = {
            default = pkgs.mkShell {
              packages = with pkgs; [
                cargo-edit
                fastfetch
                ascii-image-converter'
              ];

              env = {
                # Required by rust-analyzer
                RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
              };

              nativeBuildInputs = with pkgs; [
                cargo
                rustc
                rust-analyzer
                rustfmt
                clippy
                pkg-config
              ];

              buildInputs = with pkgs; [
                glib
                gexiv2 # for reading metadata
              ];
            };
          };

          packages = rec {
            wfetch = pkgs.callPackage ./package.nix {
              version =
                if self ? "shortRev" then
                  self.shortRev
                else
                  nixpkgs.lib.replaceStrings [ "-dirty" ] [ "" ] self.dirtyShortRev;
              # patched version of ascii-image-converter
              ascii-image-converter = ascii-image-converter';
            };
            ascii-image-converter = ascii-image-converter';
            default = wfetch;
          };
        };
      flake = {
        hydraJobs = {
          inherit (self) devShells packages;
        };
      };
    };
}
