# rustup target add x86_64-apple-ios # intel simulator
# rustup target add aarch64-apple-ios # actual iOS
# rustup target add aarch64-apple-ios-sim # arm simulator

# rustup target add aarch64-linux-android # Android arm 64 bits
# rustup target add x86_64-linux-android # Intel 64 bits emulator
# rustup target add armv7-linux-androideabi # Android arm 32 bits
# rustup target add i686-linux-android # Intel 32 bits emulator

ARCH_IOS_SIM = aarch64-apple-ios-sim
ARCHS_IOS = x86_64-apple-ios aarch64-apple-ios aarch64-apple-ios-sim
ARCHS_ANDROID = aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
LIB = libquery_engine.a
XCFRAMEWORK = QueryEngine.xcframework

.PHONY: clean ios android $(ARCH_IOS_SIM) $(ARCHS_IOS) $(ARCHS_ANDROID) sim nuke 

nuke:
	rm -rf ../../target

clean:
	rm -rf ios
	rm -rf android
	rm -rf simulator_fat
	mkdir simulator_fat

all: nuke ios android

################# ANDROID #################
android: clean $(ARCHS_ANDROID)
	mkdir -p android/jniLibs
	mkdir android/jniLibs/x86
	mkdir android/jniLibs/arm64-v8a
	mkdir android/jniLibs/armeabi-v7a
	mkdir android/jniLibs/x86_64
	cp include/query_engine.h android
	cp ../../target/i686-linux-android/release/libquery_engine.a android/jniLibs/x86/libquery_engine.a
	cp ../../target/aarch64-linux-android/release/libquery_engine.a android/jniLibs/arm64-v8a/libquery_engine.a
	cp ../../target/armv7-linux-androideabi/release/libquery_engine.a android/jniLibs/armeabi-v7a/libquery_engine.a
	cp ../../target/x86_64-linux-android/release/libquery_engine.a android/jniLibs/x86_64/libquery_engine.a

$(ARCHS_ANDROID): %: 
	./build-android-target.sh $@

################# iOS #################
ios: clean $(XCFRAMEWORK)

sim: clean
	cargo build --target $(ARCH_IOS_SIM)
	xcodebuild -create-xcframework -library ../../target/$(ARCH_IOS_SIM)/debug/libquery_engine.a -headers include -output ios/$(XCFRAMEWORK)

sim-release: clean
	cargo build --target $(ARCH_IOS_SIM) --release
	xcodebuild -create-xcframework -library ../../target/$(ARCH_IOS_SIM)/release/libquery_engine.a -headers include -output ios/$(XCFRAMEWORK)

$(ARCHS_IOS): %:
	cargo build -p query-engine-c-abi --release --target $@ 

$(XCFRAMEWORK): $(ARCHS_IOS)
	lipo -create $(wildcard ../../target/x86_64-apple-ios/release/$(LIB)) $(wildcard ../../target/aarch64-apple-ios-sim/release/$(LIB)) -output simulator_fat/libquery_engine.a
	xcodebuild -create-xcframework -library $(wildcard ../../target/aarch64-apple-ios/release/$(LIB)) -headers include -library simulator_fat/libquery_engine.a -headers include -output ios/$@