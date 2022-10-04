{ pkgs ? import <nixpkgs> {} }:

with pkgs;

let
  moldy-cargo = pkgs.writeShellScriptBin "moldy-cargo" ''
    mold -run cargo $@
  '';
  comment-out-qe-crates = pkgs.writeShellScriptBin "disable-qe-crates" ''
    sed -i 's/.*query-engine.*/#&/' Cargo.toml
  '';
  lib = pkgs.lib;
  stdenv = pkgs.stdenv;
in
mkShell {
  LIBCLANG_PATH="${pkgs.llvmPackages.libclang}/lib";
  PROTOC="${pkgs.protobuf}/bin/protoc";
  PROTOC_INCLUDE="${pkgs.protobuf}/include";

  buildInputs = with pkgs; [
    comment-out-qe-crates
    moldy-cargo
    cargo-insta

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
