{
  version,
  lib,
  installShellFiles,
  makeWrapper,
  rustPlatform,
  ascii-image-converter,
  fastfetch,
  imagemagick,
}:
rustPlatform.buildRustPackage {
  pname = "wfetch";
  inherit version;

  src = ./.;

  env.NIX_RELEASE_VERSION = version;

  cargoLock.lockFile = ./Cargo.lock;

  # create files for shell autocomplete
  nativeBuildInputs = [
    installShellFiles
    makeWrapper
  ];

  postInstall = ''
    cp -r $src/assets $out
  '';

  preFixup = ''
    installShellCompletion \
      --bash completions/*.bash \
      --fish completions/*.fish \
      --zsh completions/_*
  '';

  postFixup = ''
    wrapProgram $out/bin/wfetch \
      --prefix PATH : "${
        lib.makeBinPath [
          ascii-image-converter
          fastfetch
          imagemagick
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
