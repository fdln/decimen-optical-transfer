# Desktop app (Tauri)

`src-tauri/` wraps the site in a [Tauri v2](https://v2.tauri.app/) shell. The
pages are the web app, unmodified — protocol, codec, workers and UI run in the
system webview exactly as they do in a browser. Nothing in `shared/`, `send/`,
`receive/` or `vendor/` knows the shell exists, and the pages make no IPC
calls (the capability file grants none).

Verified on macOS only (`bundle.targets` is `app` + `dmg`). Checked there, in
the built app: page and locale routing, the camera opening through the receive
page, the WASM decoder reading a QR frame in a worker, the sender's stream and
animation export, downloads, Scan screen, and a clean CSP report.

Windows has a CI build (see [Windows installer](#windows-installer)) but is
**untested at runtime**. Linux (WebKitGTK) has neither a bundle target nor a
test — WebKitGTK's `getUserMedia` support in particular depends on how the
distribution built it.

## Build

Needs the Rust toolchain ([rustup](https://rustup.rs/)) plus the platform
prerequisites Tauri lists; Node as pinned in `.nvmrc`.

```bash
npm run tauri dev         # the shell against a Vite dev server (HMR)
npm run tauri build       # the app bundle → src-tauri/target/release/bundle/
npm run build:tauri       # just the web half → dist-tauri/
```

## Windows installer

`.github/workflows/desktop-windows.yml` builds an NSIS installer on a Windows
runner (`npx tauri build --bundles nsis`) and keeps it as the workflow
artifact `decimen-windows-installer`. It runs on a push that touches
`src-tauri/` or the workflow itself, and on demand from the Actions tab once
the file is on the default branch — a change to the pages alone does not
start it, so run it by hand for those. The bundle is picked on the command line,
so `tauri.conf.json` keeps the macOS targets.

What a green run does and does not mean:

- It proves the installer **builds**. Nothing launches it: the camera, Scan
  screen (`getDisplayMedia`), downloads and external links in WebView2 are
  unverified until someone runs it on a Windows machine.
- The installer is **unsigned**, so SmartScreen warns before running it.
- WebView2 ships with Windows 11 and current Windows 10. On a machine without
  it, the installer's default is to download the runtime — the one moment a
  network is involved. (`bundle.windows.webviewInstallMode` can embed an
  offline installer instead, at the cost of size.)

## What the shell loads: the `tauri` Vite mode

`vite build --mode tauri` is the site build (three pages, every locale tree,
the license banner) written to `dist-tauri/`, minus two hosted-site pieces:

- **No service worker / PWA.** The app is its own offline copy, and a
  precache inside a webview outlives an app update. Received media therefore
  takes the `blob:` URL path `servableMediaUrl` already has for a page no
  worker controls.
- **No self-signed HTTPS.** The webview rejects the certificate, and it loads
  the dev server from `http://localhost`, which is a secure context — the
  camera works without it. The dev server is pinned to port 5173
  (`devUrl` in `tauri.conf.json`).

`npm run build` and `dist/` are untouched by any of this.

## What the shell adds (`src-tauri/src/lib.rs`)

- **External links** (`on_navigation`, `on_new_window`) open in the system
  browser; the window never leaves the app's own pages.
- **Downloads** (`on_download`): `<a download>` — Save on a received file,
  the sender's animation export — writes to the Downloads folder (numbered
  rather than overwritten) and reveals the file.

## Camera

- **macOS**: `src-tauri/Info.plist` carries `NSCameraUsageDescription`
  (without it the OS kills the process on first camera use) and
  `Entitlements.plist` carries `com.apple.security.device.camera` for signed,
  hardened-runtime builds. The OS asks once per app identity.
- The bundled pages are served from `tauri://localhost`, which the webview
  treats as a secure context, so `navigator.mediaDevices` exists.

## CSP

`app.security.csp` in `tauri.conf.json` is the standalone policy
(`build/standalone-csp.ts`) widened only as far as a multi-file bundle needs:
`'self'` for the app's own assets, `'wasm-unsafe-eval'` for the decoder,
`blob:`/`mediastream:` for workers, previews and the camera. No http(s)
origin appears in it — "no network" stays enforced, not merely intended. (The
pages make no IPC calls, so Tauri's `ipc:` sources are left out too; add them
to `connect-src` if a page ever calls `invoke`.)

## Identity and licensing

`identifier` in `tauri.conf.json` is a placeholder for whoever ships a build
— change it before distributing. The version is read from `package.json`.
The bundle is AGPL-3.0-or-later like the rest of the project and ships
`LICENSE`, `NOTICE` and the decoder's `NOTICE.md` in its resources; the built
pages keep the license banner and the footer's source link. Those point at
the upstream repository: anyone distributing a modified build owes its
recipients the Corresponding Source of *that* build and a notice that it was
modified (AGPL §5, §6).
