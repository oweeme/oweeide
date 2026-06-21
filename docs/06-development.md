# Guía de Desarrollo

## Comandos del día a día

```bash
cd oweemedev

# Servidor de desarrollo (hot-reload)
npm run tauri dev

# Solo frontend (sin Tauri, para desarrollo UI)
npm run dev

# Type-check + build frontend
npm run build

# Verificar Rust sin compilar
cd src-tauri && cargo check

# Compilar solo el backend Rust
cd src-tauri && cargo build

# Compilar release completo con instaladores
npm run tauri build
```

## Agregar un nuevo lenguaje al editor

En `src/components/EditorArea.vue`, función `getLangExtension(language)`:

```typescript
// Para lenguajes con soporte nativo en CodeMirror 6:
import { tuLenguaje } from '@codemirror/lang-tu-lenguaje'
case 'tulenguaje': return [tuLenguaje()]

// Para lenguajes via legacy-modes:
import { tuMode } from '@codemirror/legacy-modes/mode/tu-mode'
import { StreamLanguage } from '@codemirror/language'
case 'tulenguaje': return [StreamLanguage.define(tuMode)]
```

Y en `src/composables/useEditorStore.ts`, función `detectLanguage`, agregar la extensión al mapa:

```typescript
tuext: 'tulenguaje',
```

## Agregar un nuevo comando Tauri

1. En `src-tauri/src/lib.rs`:

```rust
#[tauri::command]
async fn mi_comando(parametro: String) -> Result<String, String> {
    Ok(format!("resultado: {}", parametro))
}
```

2. Registrar en el `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... comandos existentes ...
    mi_comando,
])
```

3. Llamar desde Vue:

```typescript
import { invoke } from '@tauri-apps/api/core'
const result = await invoke<string>('mi_comando', { parametro: 'valor' })
```

## Agregar preferencias al editor

1. En `PreferencesModal.vue`: agregar el campo al formulario y al `savePrefs()` en el CustomEvent detail.

2. En `EditorArea.vue`:
   - Crear un `Compartment` nuevo
   - Incluirlo en `buildExtensions()`
   - En `onPrefsChanged()`, reconfigurarlo con `view.dispatch({ effects: miCompartment.reconfigure(...) })`

## Estructura de estilos

Las variables CSS están en `App.vue` (`:root`):

```css
--bg-darkest: #0a0c0e;
--bg-dark: #0f1115;
--bg-panel: #131619;
--bg-mid: #191d21;
--bg-hover: #1e2228;
--bg-active: #252b32;
--border: #2a3040;
--fg-bright: #e8eaf0;
--fg: #c8cbd4;
--fg-dim: #9aa0b0;
--fg-muted: #6b7280;
--accent: #2e9e87;
--font-ui: 'Inter', system-ui, sans-serif;
--font-mono: 'JetBrains Mono', 'Fira Code', monospace;
```

## Depurar errores Rust

```bash
cd oweemedev/src-tauri
RUST_LOG=debug cargo check 2>&1 | head -50
```

En modo dev, los `println!()` y `eprintln!()` de Rust aparecen en la terminal donde se ejecuta `npm run tauri dev`.

## Dependencias clave

| Paquete | Versión | Uso |
|---------|---------|-----|
| `@tauri-apps/api` | ^2 | Comunicación frontend ↔ backend |
| `@codemirror/view` | ^6 | Core del editor |
| `@codemirror/lang-*` | ^6 | Lenguajes nativos |
| `@codemirror/legacy-modes` | ^6 | Lenguajes adicionales |
| `@xterm/xterm` | ^5 | Terminal |
| `ssh2` (Rust) | 0.9 | Protocolo SFTP |
| `suppaftp` (Rust) | 5 | Protocolo FTP |
| `sqlx` (Rust) | 0.7 | MySQL/PostgreSQL/SQLite |
| `portable-pty` (Rust) | 0.8 | PTY para terminal |
| `reqwest` (Rust) | 0.12 | Peticiones HTTP (API de IA) |
