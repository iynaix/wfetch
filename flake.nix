{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
  };

  outputs =
    inputs@{
      devenv,
      flake-parts,
      nixpkgs,
      self,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ devenv.flakeModule ];
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
            default = devenv.lib.mkShell {
              inherit inputs pkgs;
              modules = [
                {
                  # https://devenv.sh/reference/options/
                  dotenv.disableHint = true;

                  packages =
                    with pkgs;
                    [
                      cargo-edit
                      fastfetch
                      pkg-config
                      glib
                      gexiv2 # for reading metadata
                    ]
                    ++ [
                      ascii-image-converter'
                    ];

                  languages.rust.enable = true;
                }
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
    };

  nixConfig = {
    extra-substituters = [ "https://wfetch.cachix.org" ];
    extra-trusted-public-keys = [ "wfetch.cachix.org-1:lFMD3l0uT/M4+WwqUXpmPAm2kvEH5xFGeIld1av0kus=" ];
  };
}
