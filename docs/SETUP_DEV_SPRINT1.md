# Setup Developpeur - Sprint 1

## Prerequis

- Windows 10/11 (64-bit)
- Node.js 20+
- Rust stable (rustup + cargo)
- Visual Studio Build Tools (Desktop development with C++)
- WebView2 Runtime

## Installation rapide

1. Installer Node.js et verifier:
   - `node -v`
   - `npm -v`
2. Installer Rust et verifier:
   - `cargo -V`
3. Depuis `apps/desktop`:
   - `npm install`
   - `npm run tauri:dev`

## Structure du repo

- `apps/desktop` : app Tauri + React
- `apps/desktop/src-tauri` : backend desktop Rust
- `crates/core` : logique core Rust partagee
- `packages/ui` : librairie UI partagee (placeholder)

## Commandes utiles

- Frontend dev: `npm run dev`
- Desktop dev: `npm run tauri:dev`
- Frontend build: `npm run build`
- Rust check: `cargo check --workspace`

## Troubleshooting

- Si `node` introuvable: redemarrer terminal apres install Node.
- Si `cargo` introuvable: verifier `rustup` et PATH.
- Si Tauri ne compile pas: verifier Build Tools C++ et WebView2.
