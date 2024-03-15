# rustup target add x86_64-apple-ios # intel simulator
# rustup target add aarch64-apple-ios # actual iOS
# rustup target add aarch64-apple-ios-sim # arm simulator

# rustup target add aarch64-linux-android # Android arm 64 bits
# rustup target add x86_64-linux-android # Intel 64 bits emulator
# rustup target add armv7-linux-androideabi # Android arm 32 bits
# rustup target add i686-linux-android # Intel 32 bits emulator

ARCH_IOS_SIM = aarch64-apple-ios-sim
ARCHS_IOS = x86_64-apple-ios aarch64-apple-ios aarch64-apple-ios-sim
# ARCHS_IOS = aarch64-apple-ios aarch64-apple-ios-sim
ARCHS_ANDROID = aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
LIB = libquery_engine.a
XCFRAMEWORK = QueryEngine.xcframework

.PHONY: clean ios android $(ARCH_IOS_SIM) $(ARCHS_IOS) $(ARCHS_ANDROID) sim copy-ios nuke

nuke:
	rm -rf ../../target

clean:
	rm -rf QueryEngine.xcframework
	rm -rf simulator_fat
	mkdir simulator_fat
	# rm -rf include
	# mkdir include

all: nuke ios android

################# ANDROID #################
android: clean $(ARCHS_ANDROID)
	./copy-android.sh

$(ARCHS_ANDROID): %:
	./build-android-target.sh $@

################# iOS #################
ios: clean $(XCFRAMEWORK)

sim: clean
	cargo build --target $(ARCH_IOS_SIM)
	xcodebuild -create-xcframework -library ../../target/$(ARCH_IOS_SIM)/debug/libquery_engine.a -headers include -output $(XCFRAMEWORK)
	./copy-ios.sh

sim-release: clean
	cargo build --target $(ARCH_IOS_SIM) --release
	xcodebuild -create-xcframework -library ../../target/$(ARCH_IOS_SIM)/release/libquery_engine.a -headers include -output $(XCFRAMEWORK)
	./copy-ios.sh

$(ARCHS_IOS): %:
	cargo build --release --target $@ 

$(XCFRAMEWORK): $(ARCHS_IOS)
# Library is too large to be published with all the targets on NPM. Therefore we are not shipping the intel version (for older intel macs)
	lipo -create $(wildcard ../../target/x86_64-apple-ios/release/$(LIB)) $(wildcard ../../target/aarch64-apple-ios-sim/release/$(LIB)) -output simulator_fat/libquery_engine.a
	xcodebuild -create-xcframework -library $(wildcard ../../target/aarch64-apple-ios/release/$(LIB)) -headers include -library simulator_fat/libquery_engine.a -headers include -output $@
	# xcodebuild -create-xcframework -library $(wildcard ../../target/aarch64-apple-ios/release/$(LIB)) -headers include -library $(wildcard ../../target/aarch64-apple-ios-sim/release/$(LIB)) -headers include -output $@
	./copy-ios.sh