{ pkgs ? import <nixpkgs> {} }:

with pkgs;

let
  moldy-cargo = pkgs.writeShellScriptBin "moldy-cargo" ''
    mold -run cargo $@
  '';
  lib = pkgs.lib;
  stdenv = pkgs.stdenv;
in
mkShell {
  LIBCLANG_PATH="${pkgs.llvmPackages.libclang}/lib";
  PROTOC="${pkgs.protobuf}/bin/protoc";
  PROTOC_INCLUDE="${pkgs.protobuf}/include";

  buildInputs = with pkgs; [
    moldy-cargo

    gcc
    openssl
    pkg-config
    clangStdenv
    llvmPackages.libclang
    kerberos
    protobuf

    rust-bin.stable.latest.default
    
  ] ++ lib.optionals stdenv.isLinux [ mold ] ++ lib.optionals stdenv.isDarwin [ darwin.apple_sdk.frameworks.Security ];
}
