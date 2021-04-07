{ pkgs ? import <nixpkgs> {} }:

with pkgs;

mkShell {
  LIBCLANG_PATH="${pkgs.llvmPackages.libclang}/lib";
  PROTOC="${pkgs.protobuf}/bin/protoc";
  PROTOC_INCLUDE="${pkgs.protobuf}/include";
  JAVA_HOME="${pkgs.jdk8}/lib/openjdk";
  # Oh lord...
  shellHook = '' rm -rf .direnv/openjdk && ln -s ${pkgs.jdk8}/lib/openjdk .direnv/openjdk '';
  buildInputs = with pkgs; [
    sbt
    sbt-extras
    jdk8
    openssl
    pkg-config
    clangStdenv
    llvmPackages.libclang
    kerberos
    protobuf
  ];
}
