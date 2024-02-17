{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
  };

  outputs =
    {
      nixpkgs,
      devenv,
      systems,
      ...
    }@inputs:
    let
      forEachSystem =
        function:
        nixpkgs.lib.genAttrs [ "x86_64-linux" ] (system: function nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forEachSystem (
        pkgs: {
          default = devenv.lib.mkShell {
            inherit inputs pkgs;
            modules = [
              {
                # https://devenv.sh/reference/options/
                dotenv.disableHint = true;

                languages.rust.enable = true;
              }
            ];
          };
        }
      );

      packages = forEachSystem (
        pkgs: rec {
          wfetch = pkgs.callPackage ./package.nix { };
          default = wfetch;
        }
      );
    };
}
