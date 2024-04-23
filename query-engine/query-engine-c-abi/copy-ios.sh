#!/bin/sh

set -ex

TARGET_DIR=../../../react-native-prisma

# This one is not actually necessary but XCode picks it up and mixes up versions
cp ./include/query_engine.h $TARGET_DIR/cpp/query_engine.h

cp -R QueryEngine.xcframework $TARGET_DIR

# pingme "✅ Prisma iOS Finished"