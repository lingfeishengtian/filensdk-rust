# /bin/bash

FRAMEWORK_NAME="FilenSDK"
LIB_NAME="filensdk"

# Compile mac, ios and ios-sim
cargo build --release
cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim

# Generate bindings for swift
cargo run --bin uniffi-bindgen generate --library target/release/lib${LIB_NAME}.dylib --language swift --out-dir swiftuniffi

# Phase to create the actual framework
rm -rf "${FRAMEWORK_NAME}.xcframework"

mv ./swiftuniffi/${LIB_NAME}FFI.modulemap ./swiftuniffi/module.modulemap

xcodebuild -create-xcframework \
        -library ./target/aarch64-apple-ios-sim/release/lib${LIB_NAME}.a -headers ./swiftuniffi \
        -library ./target/aarch64-apple-ios/release/lib${LIB_NAME}.a -headers ./swiftuniffi \
        -library ./target/release/lib${LIB_NAME}.a -headers ./swiftuniffi \
        -output "${FRAMEWORK_NAME}.xcframework"