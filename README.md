# Waifudex

Waifudex is a Tauri v2 desktop mascot shell that visualizes Codex Agent execution state. The current repository is a pre-MVP foundation with a Svelte 5 frontend, a Tauri v2 shell, and placeholder Pixi rendering.

## Runtime Setup

This project uses [`mise`](https://mise.jdx.dev/) as the single source of truth for local runtimes.

```bash
mise trust
mise install
eval "$(mise activate zsh)"
```

Pinned tool versions:

- `node`: `24.14.0`
- `pnpm`: `10.32.1`
- `rust`: `1.94.0`

After activation, keep using the normal project commands.

## Commands

```bash
pnpm install
pnpm dev
pnpm check
pnpm test -- --run
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
pnpm tauri dev
pnpm tauri build
```

## Environment Notes

The confirmed development environment is Ubuntu 24.04 on WSL. Tauri builds require system packages that `mise` does not manage, including:

- `build-essential`
- `pkg-config`
- `libwebkit2gtk-4.1-dev`
- `librsvg2-dev`
