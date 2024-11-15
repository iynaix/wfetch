{
  version,
  lib,
  installShellFiles,
  makeWrapper,
  rustPlatform,
  pkg-config,
  ascii-image-converter,
  fastfetch,
  glib,
  gexiv2,
  iynaixos ? false,
}:
rustPlatform.buildRustPackage {
  pname = "wfetch";
  inherit version;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.difference ./. (
      # don't include in build
      lib.fileset.unions [
        ./README.md
        ./LICENSE
        # ./PKGBUILD
      ]
    );
  };

  env.NIX_RELEASE_VERSION = version;

  cargoLock.lockFile = ./Cargo.lock;

  cargoBuildFlags =
    [
      "--no-default-features"
      "--features"
      "nixos"
    ]
    ++ lib.optionals iynaixos [
      "--features"
      "iynaixos"
    ];

  # create files for shell autocomplete
  nativeBuildInputs = [
    installShellFiles
    makeWrapper
    pkg-config
  ];

  buildInputs = lib.optionals iynaixos [
    glib
    gexiv2
  ];

  postInstall = ''
    cp -r $src/assets $out

    installShellCompletion --cmd wfetch \
      --bash <($out/bin/wfetch --generate bash) \
      --fish <($out/bin/wfetch --generate fish) \
      --zsh <($out/bin/wfetch --generate zsh)

    installManPage target/man/*
  '';

  postFixup = ''
    wrapProgram $out/bin/wfetch \
      --prefix PATH : "${
        lib.makeBinPath [
          ascii-image-converter
          fastfetch
        ]
      }"
  '';

  meta = with lib; {
    description = "iynaix's custom fetch";
    homepage = "https://github.com/iynaix/dotfiles";
    mainProgram = "wfetch";
    license = licenses.mit;
    maintainers = [ maintainers.iynaix ];
  };
}
