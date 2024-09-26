# wfetch

wfetch is an opinionated command-line fetch tool for displaying system information in a pretty way. It is written in Rust and is a wrapper around [fastfetch](https://github.com/fastfetch-cli/fastfetch).

<img src="https://i.imgur.com/qHGMWzW.png" />

## Installation

```nix
{
  inputs.wfetch = {
    url = "github:iynaix/wfetch";
    inputs.nixpkgs.follows = "nixpkgs"; # override this repo's nixpkgs snapshot
  };
}
```

Then, include it in your `environment.systemPackages` or `home.packages` by referencing the input:
```
inputs.wfetch.packages.${pkgs.system}.default
```

Alternatively, it can also be run directly:

```
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
          Show section of wallpaper
          (supported backends: swww, swaybg, hyprpaper, gnome, cinnamon, mate)
      --wallpaper-ascii [<WALLPAPER>]
          Show section of wallpaper in ascii
          (supported backends: swww, swaybg, hyprpaper, gnome, cinnamon, mate) [aliases: ascii-wallpaper, ascii]
      --challenge
          Show challenge progress
      --challenge-timestamp <TIMESTAMP>
          Start of the challenge as a UNIX timestamp in seconds [default: 1675821503]
      --challenge-years <YEARS>
          Duration of challenge in years [default: 10]
      --challenge-months <MONTHS>
          Duration of challenge in months [default: 0]
      --challenge-type <CHALLENGE_TYPE>
          Type of the challenge, e.g. emacs
      --listen
          Listen for SIGUSR2 to refresh output [aliases: socket]
      --no-color-keys
          Do not show colored keys
      --image-size <IMAGE_SIZE>
          Image size in pixels
      --ascii-size <ASCII_SIZE>
          Ascii size in characters [default: 70]
  -h, --help
          Print help
  -V, --version
          Print version
```

## Screenshots

### (default)
<img src="https://i.imgur.com/gtbUnjL.png" /><br/>

### --hollow
<img src="https://i.imgur.com/9Fxua7R.png" /><br/>

### --wallpaper
<img src="https://i.imgur.com/qHGMWzW.png" />

### --waifu
<img src="https://i.imgur.com/QbFz33S.png" />

### --waifu2
<img src="https://i.imgur.com/1PhJNDU.png" />

### --wallpaper-ascii
<img src="https://i.imgur.com/4nHd6F5.png" /><br/>

## Packaging

To build wfetch from source

- Build dependencies
    - Rust (cargo, rustc)
- Runtime dependencies
    - [fastfetch](https://github.com/fastfetch-cli/fastfetch/blob/dev/README.md)
    - [ascii-image-converter](https://github.com/TheZoraiz/ascii-image-converter)

## Hacking

Just use `nix develop`