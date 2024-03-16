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
    installShellCompletion --bash completions/*.bash
    installShellCompletion --fish completions/*.fish
    installShellCompletion --zsh completions/_*
  '';

  postFixup = ''
    wrapProgram $out/bin/wfetch \
      --prefix PATH : "${lib.makeBinPath [ ascii-image-converter ]}" \
      --prefix PATH : "${lib.makeBinPath [ fastfetch ]}" \
      --prefix PATH : "${lib.makeBinPath [ imagemagick ]}"
  '';

  meta = with lib; {
    description = "iynaix's custom fetch";
    homepage = "https://github.com/iynaix/dotfiles";
    license = licenses.mit;
    maintainers = [ maintainers.iynaix ];
  };
}
