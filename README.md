# Ferrite

A Minecraft client written in Rust from scratch. Ferrite connects to vanilla Minecraft servers, renders the world, and handles player physics  - all without relying on Mojang's Java codebase.

## Why Ferrite?

- **Performance**  - Native code with zero garbage collection pauses. GPU rendering through wgpu with direct Vulkan/DX12/Metal backends.
- **Low memory footprint**  - No JVM overhead. Runs comfortably on hardware that struggles with vanilla Java Edition.
- **Cross-platform**  - Builds natively on Windows, Linux, and macOS from a single codebase.
- **Hackable**  - Clean, modular Rust codebase. Easy to understand, modify, and extend.

## Current Status

Ferrite is in active early development, working through milestones toward a fully playable client.

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Window + GPU initialization | Done |
| 2 | Camera movement + basic rendering | Done |
| 3 | Server connection + protocol handling | Done |
| 4 | Terrain rendering with textures | Done |
| 5 | Player physics + collision | In progress |
| 6 | HUD, chat, inventory | Planned |
| 7 | Main menu, server list, settings | Planned |

## Building

Requires **Rust nightly** and vanilla 1.21.11 assets.

```bash
# Set up nightly toolchain
rustup override set nightly

# Extract vanilla assets (needed for block textures)
unzip ~/.minecraft/versions/1.21.11/1.21.11.jar -d reference/assets/

# Build
cargo build --release
```

## Running

```bash
# Connect to a server
cargo run --release -- --server localhost:25565 --username Steve

# With authentication (for online-mode servers)
cargo run --release -- \
  --server mc.example.com \
  --username Player \
  --uuid <your-uuid> \
  --access-token <your-token>
```

## Tech Stack

| Component | Crate |
|-----------|-------|
| GPU rendering | `wgpu` |
| Windowing | `winit` |
| Math | `glam` |
| Protocol | `azalea-protocol` |
| Async runtime | `tokio` |
| Textures | `image` |

## Contributing

See [CONTRIBUTING.md](.github/CONTRIBUTING.md) for setup instructions and guidelines.

## License

This project is not affiliated with Mojang or Microsoft.
