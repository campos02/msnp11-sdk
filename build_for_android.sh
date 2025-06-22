cd msnp11-sdk || exit
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -t x86 -o ./jniLibs build --release

cd ..
cargo run -p uniffi-bindgen generate --library target/aarch64-linux-android/release/libmsnp11_sdk.so --language kotlin --out-dir out
