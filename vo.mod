module github.com/vo-lang/notify

vo ^0.1.0

[extension]
name = "notify"

[extension.native]
path = "rust/target/{profile}/libvo_notify"

[[extension.native.targets]]
target = "aarch64-apple-darwin"
library = "libvo_notify.dylib"

[[extension.native.targets]]
target = "x86_64-unknown-linux-gnu"
library = "libvo_notify.so"
