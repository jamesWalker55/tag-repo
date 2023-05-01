# tagrepo

A hybrid tag-hierarchy file manager. This is extremely early in development.

## Development

Requirements:

- Rust
- Yarn
- [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)

For development, you need two terminals: one to start a server, and one to run the application alongside it.

If you encounter errors when trying to compile the app, please make sure you've followed the [Tauri prerequisites guide](https://tauri.app/v1/guides/getting-started/prerequisites).

Starting the server:

```bash
# Install packages
yarn
# Start the server
yarn dev
```

Start the program: (Run this in a new terminal)

```bash
# Compile and run the program
cargo run
```

## Code structure

The app is divided into the frontend (TypeScript) and backend (Rust). The code for the frontend is at `./src`, the code for the backend is at `./src-tauri`
