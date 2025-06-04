cd msnp11-sdk || exit
cross build --release --target x86_64-linux-android
cross build --release --target i686-linux-android
cross build --release --target armv7-linux-androideabi
cross build --release --target aarch64-linux-android

cd ..
cargo run -p uniffi-bindgen generate --library target/x86_64-linux-android/release/libmsnp11_sdk.so --language kotlin --out-dir out

cd out || exit
cp ../target/x86_64-linux-android/release/libmsnp11_sdk.so ./jniLibs/x86_64
cp ../target/i686-linux-android/release/libmsnp11_sdk.so ./jniLibs/x86
cp ../target/armv7-linux-androideabi/release/libmsnp11_sdk.so ./jniLibs/armeabi-v7a
cp ../target/aarch64-linux-android/release/libmsnp11_sdk.so ./jniLibs/arm64-v8a