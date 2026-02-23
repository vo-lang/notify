# vo-lang/notify

Vo wrapper for Rust `notify` filesystem watching.

## Module

```vo
import "github.com/vo-lang/notify"
```

## Implemented API

- `New()`
- `Create(path, recursive)`
- `Watcher.Watch(path, recursive)`
- `Watcher.Unwatch(path)`
- `Watcher.Poll(max)`
- `Watcher.Close()`

## Build

```bash
cargo check --manifest-path rust/Cargo.toml
```
