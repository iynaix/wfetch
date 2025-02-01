# wfetch

wfetch is an opinionated command-line fetch tool for displaying system information in a pretty way. It is written in Rust and is a wrapper around [fastfetch](https://github.com/fastfetch-cli/fastfetch).

<!-- 110026463_p0.webp -->
<img src="https://i.imgur.com/D5H2xG6.png" />

## Installation

Add wfetch to your `inputs` in your `flake.nix`:
```nix
{
  inputs.wfetch.url = "github:iynaix/wfetch";
}
```

A [wfetch cachix](https://wfetch.cachix.org) is also available, providing prebuilt binaries. To use it, add the following to your configuration:
```nix
{
  nix.settings = {
    substituters = ["https://wfetch.cachix.org"];
    trusted-public-keys = ["wfetch.cachix.org-1:lFMD3l0uT/M4+WwqUXpmPAm2kvEH5xFGeIld1av0kus="];
  };
}
```

> [!Warning]
> Overriding the `wfetch` input using a `inputs.nixpkgs.follows` invalidates the cache and will cause the package to be rebuilt.

Then, include it in your `environment.systemPackages` or `home.packages` by referencing the input:
```nix
inputs.wfetch.packages.${pkgs.system}.default
```

Alternatively, it can also be run directly:

```sh
nix run github:iynaix/wfetch
```

## Usage

```console
$ wfetch --help
wfetch is an opinionated command-line fetch tool for displaying system information in a pretty way

Usage: wfetch [OPTIONS]

Options:
      --hollow
          Show hollow NixOS logo

      --waifu
          Show waifu NixOS logo with dynamic colors

      --waifu2
          Show waifu NixOS logo 2 with dynamic colors

      --wallpaper [<WALLPAPER>]
          Show section of wallpaper, use "-" for stdin
          (supported backends: swww, swaybg, hyprpaper, gnome, cinnamon, mate)

      --crop <CROP_AREA>
          Specify square area of the wallpaper to display in the format WxH+X+Y

      --wallpaper-ascii [<WALLPAPER>]
          Show section of wallpaper in ascii, use "-" for stdin
          (supported backends: swww, swaybg, hyprpaper, gnome, cinnamon, mate)

          [aliases: ascii-wallpaper, ascii]

      --challenge
          Show challenge progress

      --challenge-timestamp <TIMESTAMP>
          Start of the challenge as a UNIX timestamp in seconds

          [default: 1675821503]

      --challenge-years <YEARS>
          Duration of challenge in years

          [default: 10]

      --challenge-months <MONTHS>
          Duration of challenge in months

          [default: 0]

      --challenge-type <CHALLENGE_TYPE>
          Type of the challenge, e.g. emacs

      --listen
          Listen for SIGUSR2 to refresh output

          [aliases: socket]

      --no-color-keys
          Do not show colored keys

      --image-size <IMAGE_SIZE>
          Image size in pixels

      --ascii-size <ASCII_SIZE>
          Ascii size in characters

          [default: 70]

      --scale <SCALE>
          Scale factor for high DPI displays

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Screenshots

### (default)
<img src="https://i.imgur.com/X0m8dVt.png" /><br/>

### --hollow
<img src="https://i.imgur.com/jtZjItL.png" /><br/>

### --wallpaper
<!-- 110026463_p0.webp -->
<img src="https://i.imgur.com/D5H2xG6.png" />

### --waifu
<img src="https://i.imgur.com/otI2IL8.png" />

### --waifu2
<img src="https://i.imgur.com/FVy3Mwt.png" />

### --wallpaper-ascii
<!-- wallhaven-5g67o9.webp -->
<img src="https://i.imgur.com/wzX9xUQ.png" /><br/>

## Packaging

To build wfetch from source

- Build dependencies
    - Rust (cargo, rustc)
- Runtime dependencies
    - [fastfetch](https://github.com/fastfetch-cli/fastfetch/blob/dev/README.md)
    - [ascii-image-converter](https://github.com/TheZoraiz/ascii-image-converter)

## Hacking

Just use `nix develop`
