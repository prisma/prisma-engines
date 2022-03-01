{ pkgs ? import <nixpkgs> {} }:

with pkgs;

let
  moldy-cargo = pkgs.writeShellScriptBin "moldy-cargo" ''
    mold -run cargo $@
  '';
in
mkShell {
  LIBCLANG_PATH="${pkgs.llvmPackages.libclang}/lib";
  PROTOC="${pkgs.protobuf}/bin/protoc";
  PROTOC_INCLUDE="${pkgs.protobuf}/include";

  buildInputs = with pkgs; [
    mold # much faster linker
    moldy-cargo

    gcc
    openssl
    pkg-config
    clangStdenv
    llvmPackages.libclang
    kerberos
    protobuf

    rust-bin.stable.latest.default
  ];
}
