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
      --version
          print version information and exit
      --hollow
          show hollow NixOS logo
      --waifu
          show waifu NixOS logo with dynamic colors
      --waifu2
          show waifu NixOS logo 2 with dynamic colors
      --wallpaper [<WALLPAPER>]
          show section of wallpaper
          (supported backends: swww, swaybg, hyprpaper, gnome, cinnamon, mate)
      --wallpaper-ascii [<WALLPAPER>]
          show section of wallpaper in ascii
          (supported backends: swww, swaybg, hyprpaper, gnome, cinnamon, mate)
      --challenge
          show challenge progress
      --challenge-timestamp <TIMESTAMP>
          start of the challenge as a UNIX timestamp in seconds [default: 1675821503]
      --challenge-years <YEARS>
          duration of challenge in years [default: 10]
      --challenge-months <MONTHS>
          duration of challenge in months [default: 0]
      --challenge-type <CHALLENGE_TYPE>
          type of the challenge, e.g. emacs
      --listen
          listen for SIGUSR2 to refresh output
      --no-color-keys
          do not show colored keys
      --image-size <IMAGE_SIZE>
          image size in pixels
      --ascii-size <ASCII_SIZE>
          ascii size in characters [default: 70]
  -h, --help
          Print help
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
    1. Rust (cargo, rustc)
- Runtime dependencies
    1. [imagemagick](https://imagemagick.org/)
    1. [fastfetch](https://github.com/fastfetch-cli/fastfetch/blob/dev/README.md)
    1. [ascii-image-converter](https://github.com/TheZoraiz/ascii-image-converter)

## Hacking

Just use `nix develop`