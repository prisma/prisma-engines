{ pkgs ? import <nixpkgs> {} }:

with pkgs;

mkShell {
  LIBCLANG_PATH="${pkgs.llvmPackages.libclang}/lib";
  PROTOC="${pkgs.protobuf}/bin/protoc";
  PROTOC_INCLUDE="${pkgs.protobuf}/include";
  buildInputs = with pkgs; [
    sbt
    sbt-extras
    jdk
    openssl
    pkg-config
    clangStdenv
    llvmPackages.libclang
    kerberos
    protobuf
  ];
}
