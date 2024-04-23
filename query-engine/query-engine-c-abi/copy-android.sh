#! /bin/bash

TARGET_DIR=../../../react-native-prisma

mkdir -p $TARGET_DIR/android/jniLibs
mkdir -p $TARGET_DIR/android/jniLibs/x86
mkdir -p $TARGET_DIR/android/jniLibs/x86_64
mkdir -p $TARGET_DIR/android/jniLibs/arm64-v8a
mkdir -p $TARGET_DIR/android/jniLibs/armeabi-v7a

cp ../../target/i686-linux-android/release/libquery_engine.a $TARGET_DIR/android/jniLibs/x86/libquery_engine.a
cp ../../target/aarch64-linux-android/release/libquery_engine.a $TARGET_DIR/android/jniLibs/arm64-v8a/libquery_engine.a
cp ../../target/armv7-linux-androideabi/release/libquery_engine.a $TARGET_DIR/android/jniLibs/armeabi-v7a/libquery_engine.a
cp ../../target/x86_64-linux-android/release/libquery_engine.a $TARGET_DIR/android/jniLibs/x86_64/libquery_engine.a

cp ./include/query_engine.h $TARGET_DIR/cpp/query_engine.h

# pingme "✅ Android compilation ready"