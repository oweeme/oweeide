# OweemeIDE — Visión General

OweemeIDE es un entorno de desarrollo integrado (IDE) de escritorio construido con:

- **Frontend**: Vue 3 + TypeScript + Vite
- **Backend nativo**: Rust via Tauri 2
- **Editor de código**: CodeMirror 6
- **Terminal**: xterm.js con PTY nativo (portable-pty)
- **Base de datos**: SQLx (MySQL, PostgreSQL, SQLite)
- **FTP/SFTP**: ssh2 + suppaftp

## Estructura del proyecto

```
oweeide/
├── docs/                    # Esta documentación
└── oweemedev/               # Proyecto Tauri
    ├── src/                 # Frontend Vue 3
    │   ├── App.vue          # Raíz: layout, activity bar, sidebar
    │   ├── components/
    │   │   ├── EditorArea.vue      # Gestor de tabs + CodeMirror
    │   │   ├── FileExplorer.vue    # Árbol de archivos
    │   │   ├── SearchPanel.vue     # Búsqueda en archivos
    │   │   ├── Terminal.vue        # Terminal integrada (xterm.js + PTY)
    │   │   ├── DatabasePanel.vue   # Sidebar de conexiones DB
    │   │   ├── DatabaseView.vue    # Vista tabla + editor SQL
    │   │   ├── FtpPanel.vue        # Sidebar de conexiones FTP/SFTP
    │   │   ├── FtpView.vue         # Cliente dual-pane FTP/SFTP
    │   │   ├── AiPanel.vue         # Asistente IA (Claude, OpenAI, Gemini)
    │   │   ├── PreferencesModal.vue # Configuración del editor
    │   │   └── StatusBar.vue       # Barra de estado inferior
    │   └── composables/
    │       └── useEditorStore.ts   # Estado global (tabs, rutas, cursor)
    └── src-tauri/
        ├── src/lib.rs       # Todos los comandos Tauri (Rust)
        ├── Cargo.toml       # Dependencias Rust
        └── tauri.conf.json  # Configuración Tauri (bundle, ventana, etc.)
```

## Funcionalidades principales

| Área | Características |
|------|----------------|
| Editor | CodeMirror 6, syntax highlighting multi-lenguaje, preferencias live |
| Explorador | Árbol de archivos, archivos ocultos toggle, crear/renombrar/eliminar |
| Búsqueda | Búsqueda full-text en archivos del proyecto |
| Terminal | PTY nativo, múltiples sesiones, scroll history |
| Base de datos | MySQL/PostgreSQL/SQLite, vista tabla, editor SQL con intellisense |
| FTP/SFTP | Dual-pane, multi-select, subida/descarga recursiva, preview inline |
| IA | Claude (Anthropic), OpenAI GPT, Gemini — en sidebar |
| Preferencias | Fuente, tamaño, line-height, tab size, word wrap, auto-save |
