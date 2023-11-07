#!/bin/bash

# 获取版本号
version=$(git rev-parse --short HEAD)

# 定义平台和目标架构
platforms=("darwin-x64" "linux-x64" "windows-x64" "darwin-arm64" "linux-arm64" "windows-arm64")
targets=("x86_64-apple-darwin" "x86_64-unknown-linux-gnu" "x86_64-pc-windows-gnu" "aarch64-apple-darwin" "aarch64-unknown-linux-gnu" "aarch64-pc-windows-gnu")

# 编译
for i in ${!platforms[@]}; do
  cross build --release -p schema-engine-cli --target ${targets[$i]}
  cp target/${targets[$i]}/release/schema-engine target/release/"${platforms[$i]}$(if [[ ${platforms[$i]} == *windows* ]]; then echo .exe; fi)"
done
