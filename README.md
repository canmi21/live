# Live

A modular configuration framework with live reloading, atomic updates, and format-agnostic loading.

`live` provides a complete solution for managing application configuration in Rust. It integrates atomic storage, flexible loaders, and filesystem monitoring into a unified controller, enabling applications to react to configuration changes in real-time without restarts.

## Features

- **Atomic Storage**: Thread-safe configuration store (`Store`) using RCU semantics for wait-free reads and consistent updates.
- **Live Reloading**: Built-in filesystem monitoring (`Watcher`) that automatically detects changes and triggers reloads.
- **Format Agnostic**: Support for multiple formats (`JSON`, `TOML`, `YAML`, `Postcard`) with automatic detection and extension.
- **Secure Loading**: `FileSource` with sandbox protection against path traversal attacks.
- **Unified Controller**: The `Live<T>` controller ties everything together, providing a simple API for loading, accessing, and watching configurations.
- **Lifecycle Management**:
  - **Validation**: Integration with `validator` to ensure config validity before update.
  - **Preprocessing**: Hooks for data normalization or context injection.
  - **Debouncing**: Intelligent event coalescing to prevent redundant reloads.

## Usage Examples

Check the `examples` directory for runnable code:

- **Basic Usage**: [`examples/basic.rs`](examples/basic.rs) - Demonstrates how to setup a `Live` controller, load a configuration, and watch for file changes.

## Installation

```toml
[dependencies]
live = { version = "0.3", features = ["full"] }
```

## Feature Flags

`live` is highly modular. You can enable only the features you need.

| Feature | Description |
|---------|-------------|
| `holder` | Enables the atomic storage module (`Store`). |
| `loader` | Enables the configuration loading module (`DynLoader`, `StaticLoader`). |
| `signal` | Enables the filesystem monitoring module (`Watcher`). |
| `controller` | Enables the `Live` controller (requires `holder` + `loader`). |
| `full` | Enables all features above. |

## License

Released under the MIT License © 2026 [Canmi](https://github.com/canmi21)
