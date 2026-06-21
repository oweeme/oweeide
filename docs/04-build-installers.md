# Compilar e Instalar OweemeIDE

## Requisitos previos

### Todos los sistemas
```bash
# Node.js 18+ y npm
node --version   # >= 18
npm --version    # >= 9

# Rust toolchain estable
rustup update stable
rustc --version  # >= 1.77
```

### Linux
```bash
# Dependencias del sistema (Ubuntu/Debian)
sudo apt update && sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libssl-dev \
  pkg-config \
  libssh2-1-dev   # para el módulo ssh2/SFTP

# Para generar AppImage necesitas FUSE:
sudo apt install -y fuse libfuse2
```

### Windows
- Instalar [Microsoft Visual C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) o Visual Studio con "Desktop development with C++"
- Instalar [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (ya incluido en Windows 10/11 actualizados)
- Instalar [OpenSSL para Windows](https://slproweb.com/products/Win32OpenSSL.html) o via vcpkg:
  ```powershell
  vcpkg install openssl:x64-windows
  ```

---

## Desarrollo local

```bash
cd oweemedev
npm install
npm run tauri dev
```

---

## Compilar para producción

### Todos los targets del sistema actual

```bash
cd oweemedev
npm run tauri build
```

Los instaladores se generan en:
```
oweemedev/src-tauri/target/release/bundle/
```

---

## Targets específicos

### Linux — .deb (Debian/Ubuntu)

```bash
npm run tauri build -- --bundles deb
```

Salida: `target/release/bundle/deb/oweemedev_0.1.0_amd64.deb`

Instalar:
```bash
sudo dpkg -i oweemedev_0.1.0_amd64.deb
# o
sudo apt install ./oweemedev_0.1.0_amd64.deb
```

### Linux — AppImage (universal, sin instalación)

```bash
npm run tauri build -- --bundles appimage
```

Salida: `target/release/bundle/appimage/oweemedev_0.1.0_amd64.AppImage`

Ejecutar directamente:
```bash
chmod +x oweemedev_0.1.0_amd64.AppImage
./oweemedev_0.1.0_amd64.AppImage
```

### Linux — RPM (Fedora/RHEL)

```bash
npm run tauri build -- --bundles rpm
```

### Windows — NSIS Installer (.exe)

En un sistema Windows:
```powershell
npm run tauri build -- --bundles nsis
```

Salida: `target/release/bundle/nsis/OweemeIDE_0.1.0_x64-setup.exe`

### Windows — MSI

```powershell
npm run tauri build -- --bundles msi
```

Salida: `target/release/bundle/msi/OweemeIDE_0.1.0_x64_en-US.msi`

---

## Cross-compilation (desde Linux → Windows)

Requiere el toolchain de Windows en Linux:

```bash
# Instalar target Windows
rustup target add x86_64-pc-windows-gnu

# Instalar mingw-w64
sudo apt install gcc-mingw-w64-x86-64

# Compilar
npm run tauri build -- --target x86_64-pc-windows-gnu
```

> **Nota**: La cross-compilation puede tener problemas con OpenSSL y libssh2. Se recomienda compilar en Windows nativo para los instaladores de Windows.

---

## Configuración del bundle (tauri.conf.json)

```json
{
  "productName": "OweemeIDE",
  "version": "0.1.0",
  "identifier": "com.oweeme.oweemedev",
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

Para cambiar la versión, editar `version` en `tauri.conf.json` y también en `Cargo.toml` del `src-tauri/`.

---

## Flatpak (distribución en Flathub)

Para publicar en Flathub se necesita:

1. Crear el manifest `com.oweeme.OweemeIDE.yml`:

```yaml
app-id: com.oweeme.OweemeIDE
runtime: org.gnome.Platform
runtime-version: '46'
sdk: org.gnome.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
  - org.freedesktop.Sdk.Extension.node20
command: oweemeide
finish-args:
  - --share=network
  - --share=ipc
  - --socket=wayland
  - --socket=fallback-x11
  - --filesystem=home
  - --device=dri
modules:
  - name: oweemeide
    buildsystem: simple
    build-commands:
      - npm ci --prefix oweemedev
      - npm run tauri build --prefix oweemedev -- --bundles deb
      - install -Dm755 oweemedev/src-tauri/target/release/oweemedev /app/bin/oweemeide
```

2. Publicar en Flathub requiere un repositorio público en GitHub y un PR al repo [flathub/flathub](https://github.com/flathub/flathub).

---

## GitHub Actions — CI/CD automático

Crear `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    strategy:
      matrix:
        include:
          - os: ubuntu-22.04
            targets: deb,appimage
          - os: windows-latest
            targets: nsis,msi

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Linux deps
        if: matrix.os == 'ubuntu-22.04'
        run: |
          sudo apt update
          sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
            librsvg2-dev patchelf libssl-dev libssh2-1-dev

      - name: Install npm deps
        working-directory: oweemedev
        run: npm ci

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          projectPath: oweemedev
          tagName: ${{ github.ref_name }}
          releaseName: OweemeIDE ${{ github.ref_name }}
          releaseBody: |
            Versión ${{ github.ref_name }} de OweemeIDE
          releaseDraft: true
          prerelease: false
          args: --bundles ${{ matrix.targets }}
```

### Cómo hacer un release

```bash
# Actualizar versión en tauri.conf.json y Cargo.toml
# Luego:
git add .
git commit -m "chore: version 0.2.0"
git tag v0.2.0
git push origin main --tags
```

GitHub Actions compilará automáticamente para Linux y Windows, y creará el Release con los instaladores adjuntos.

---

## Iconos personalizados

Para cambiar los iconos del instalador:

```bash
cd oweemedev
# Coloca tu icono en src-tauri/icons/app-icon.png (1024x1024 px)
npm run tauri icon src-tauri/icons/app-icon.png
```

Esto regenera automáticamente todos los tamaños necesarios.
