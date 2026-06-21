# Subir a GitHub y Publicar Instaladores

## 1. Inicializar repositorio Git

```bash
cd /home/oweeme/Dev/rust/oweeide

git init
git add .
git commit -m "feat: OweemeIDE v0.1.0 — IDE de escritorio Tauri 2 + Vue 3"
```

## 2. Crear repositorio en GitHub

```bash
# Con GitHub CLI (recomendado)
gh repo create oweemeide --public --description "IDE de escritorio ligero construido con Tauri 2, Vue 3 y Rust"

# Conectar y subir
git remote add origin https://github.com/TU_USUARIO/oweemeide.git
git branch -M main
git push -u origin main
```

O manualmente en [github.com/new](https://github.com/new).

## 3. Archivo .gitignore

Crear `/home/oweeme/Dev/rust/oweeide/.gitignore`:

```gitignore
# Node
oweemedev/node_modules/
oweemedev/dist/

# Rust / Cargo
oweemedev/src-tauri/target/

# Tauri generados
oweemedev/src-tauri/gen/

# OS
.DS_Store
Thumbs.db

# Editor
.vscode/
.idea/
*.swp
```

## 4. Crear el workflow de GitHub Actions

Crear el directorio y archivo:
```bash
mkdir -p .github/workflows
```

Luego crear `.github/workflows/release.yml` con el contenido del documento `04-build-installers.md`.

## 5. Hacer el primer release

```bash
# Asegurarse de que la versión en tauri.conf.json y Cargo.toml es correcta
# Luego crear el tag:
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions se activará automáticamente con el tag `v*` y:
1. Compilará en Ubuntu 22.04 → genera `.deb` y `.AppImage`
2. Compilará en Windows → genera `.exe` (NSIS) y `.msi`
3. Creará un GitHub Release como draft con todos los instaladores adjuntos
4. Puedes editar el Release en GitHub y publicarlo cuando esté listo

## 6. Secrets necesarios

GitHub Actions usa `GITHUB_TOKEN` automáticamente — no necesitas configurar nada adicional para publicar releases.

Si en el futuro necesitas firmar los instaladores (recomendado para distribución), configura en GitHub → Settings → Secrets:
- `TAURI_SIGNING_PRIVATE_KEY` — clave privada para el auto-updater de Tauri
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — passphrase

Generar claves:
```bash
npm run tauri signer generate -- -w ~/.tauri/myapp.key
```

## 7. Estructura de releases recomendada

```
Releases/
├── v0.1.0
│   ├── OweemeIDE_0.1.0_amd64.deb          (Linux Debian/Ubuntu)
│   ├── OweemeIDE_0.1.0_amd64.AppImage     (Linux universal)
│   ├── OweemeIDE_0.1.0_x64-setup.exe      (Windows NSIS)
│   └── OweemeIDE_0.1.0_x64_en-US.msi     (Windows MSI)
└── v0.2.0
    └── ...
```

## 8. Badge para el README

```markdown
[![Release](https://img.shields.io/github/v/release/TU_USUARIO/oweemeide)](https://github.com/TU_USUARIO/oweemeide/releases)
[![Linux](https://img.shields.io/badge/Linux-.deb%20%7C%20AppImage-blue)](https://github.com/TU_USUARIO/oweemeide/releases)
[![Windows](https://img.shields.io/badge/Windows-.exe%20%7C%20.msi-blue)](https://github.com/TU_USUARIO/oweemeide/releases)
```
