{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
  };

  outputs =
    inputs@{
      flake-parts,
      nixpkgs,
      self,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.devenv.flakeModule ];
      systems = nixpkgs.lib.systems.flakeExposed;

      perSystem =
        {
          # config,
          # self',
          # inputs',
          pkgs,
          # system,
          ...
        }:
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
            default = inputs.devenv.lib.mkShell {
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
            wfetch-iynaixos = wfetch.override { iynaixos = true; };
          };
        };
    };
}
