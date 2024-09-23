{
  version,
  lib,
  installShellFiles,
  makeWrapper,
  rustPlatform,
  ascii-image-converter,
  fastfetch,
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

  # create files for shell autocomplete
  nativeBuildInputs = [
    installShellFiles
    makeWrapper
  ];

  postInstall = ''
    cp -r $src/assets $out

    installShellCompletion --cmd wfetch \
      --bash <($out/bin/wfetch --generate bash) \
      --fish <($out/bin/wfetch --generate fish) \
      --zsh <($out/bin/wfetch --generate zsh)
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
