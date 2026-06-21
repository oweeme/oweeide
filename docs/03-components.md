# Componentes Vue

## App.vue

Raíz de la aplicación. Gestiona:
- **Activity bar** (izquierda): botones de Explorer, Búsqueda, Base de datos, FTP/SFTP, IA, Preferencias
- **Sidebar** (`<aside>`): muestra el panel activo (FileExplorer, SearchPanel, DatabasePanel, FtpPanel, AiPanel)
- **Panel inferior**: Terminal con xterm.js
- **Área principal**: `<EditorArea>` con los tabs

Props/refs clave:
- `sidebarView`: `'explorer' | 'search' | 'database' | 'ftp' | 'ai'`
- `showSidebar`: boolean
- `showPanel`: boolean (terminal visible)

---

## EditorArea.vue

Gestiona los tabs y el editor CodeMirror 6.

- Renderiza un tab por cada `store.state.tabs[]`
- Para `type === 'code'`: monta CodeMirror con el lenguaje detectado
- Para `type === 'image'`: `<img>` con `convertFileSrc()`
- Para `type === 'database'`: `<DatabaseView>`
- Para `type === 'ftp'`: `<FtpView>`

**Preferencias live**: escucha el evento `prefs-changed` (CustomEvent) emitido por `PreferencesModal`. Actualiza las extensiones de CodeMirror vía Compartments sin recrear el editor.

**Auto-save**: el `updateListener` de CodeMirror llama a `store.saveFile()` si `autoSave` está activo.

---

## FileExplorer.vue

Árbol de archivos recursivo. 

- Botón de ojo (👁) alterna `showHidden` — persiste en `localStorage('explorer_show_hidden')`
- Click en archivo → `store.openFile(path)`
- Menú contextual (clic derecho): crear archivo, crear carpeta, renombrar, eliminar
- Carga con `list_dir(path, { showHidden })` al abrir cada nodo

---

## Terminal.vue

xterm.js con PTY nativo.

- `pty_create(cols, rows)` al montar → obtiene `ptyId`
- Poll de lectura: `setInterval(() => pty_read(ptyId))` cada 50ms
- `fit-addon` para redimensionar automático
- Resize observer → `pty_resize(ptyId, cols, rows)`
- `pty_destroy(ptyId)` al desmontar

**Fix importante**: `Math.max(10, Math.floor(dims?.cols ?? 80))` porque `proposeDimensions()` puede retornar null cuando el panel está oculto.

---

## DatabasePanel.vue

Sidebar de conexiones DB. Guarda conexiones en `localStorage('db_connections')`.

- Formulario: driver (MySQL/PostgreSQL/SQLite), host, puerto, usuario, contraseña, base de datos
- Al conectar: `db_connect(...)` → lista tablas con `db_list_tables()`
- Click en tabla → `store.openDbTable(connId, tableName, connName, driver)`

---

## DatabaseView.vue

Vista principal de una tabla o consulta SQL.

- Panel superior: grid de datos con paginación
- Panel inferior: editor SQL (CodeMirror 6) con:
  - Dialecto según driver (MySQL/PostgreSQL/SQLite)
  - Autocomplete de tablas y columnas del schema real
  - `Ctrl+Enter` ejecuta la consulta
- `db_query_table()` para vista de tabla, `db_execute()` para SQL libre

---

## FtpPanel.vue

Sidebar de conexiones FTP/SFTP. Guarda en `localStorage('ftp_connections')`.

- Formulario: protocolo (SFTP/FTP), host, puerto, usuario, contraseña, ruta raíz, tipo auth (password/SSH key)
- `remote_connect(...)` → abre tab FTP con `store.openFtpConn()`
- Al desconectar: `remote_disconnect()` + `window.dispatchEvent(new CustomEvent('ftp-disconnected', { detail: connId }))`
- `connectedIds`: Set de IDs actualmente conectados — persiste solo en memoria (no en localStorage)

---

## FtpView.vue

Cliente FTP/SFTP dual-pane. Props: `connId`, `connName`, `protocol`.

### Panel LOCAL (izquierdo)
- Navegación con `list_dir()` (siempre `showHidden: true`)
- **Multi-select**: `Ctrl+click` o `Espacio` por teclado, `Set<string>`
- Badge "N sel." cuando hay selección
- Preview/edición inline de archivos locales
- Botones: atrás, subir nivel, home (raíz del proyecto), recargar

### Panel SERVIDOR (derecho)
- Navegación con `remote_list_dir()`
- **Multi-select**: igual que local — `Ctrl+click`, `Espacio`, `Set<string>`, badge contador
- Breadcrumbs clicables para navegación rápida
- Preview/edición inline de archivos remotos
- Acciones por fila: ver, renombrar, eliminar
- Nueva carpeta inline

### Transferencias
- **Subir**: `uploadSelected()` — recolecta todos los archivos recursivamente, crea carpetas remotas, sube con barra de progreso
- **Descargar**: `downloadSelected()` — descarga todos los archivos seleccionados (no carpetas) con barra de progreso
- Barra de progreso: `{ done, total }` animada

### Teclado (ambos paneles)
| Tecla | Local | Servidor |
|-------|-------|---------|
| ↑ / ↓ | Mueve cursor | Mueve cursor |
| Espacio | Toggle selección | Toggle selección |
| Enter | Entrar carpeta / preview | Entrar carpeta / preview |
| Backspace / ← | Volver atrás | Volver atrás |
| Ctrl+A | Seleccionar todo | Seleccionar todo |
| Escape | Limpiar selección | Limpiar selección |
| U | Subir seleccionados | — |
| D | — | Descargar seleccionados |
| Delete | — | Eliminar seleccionados |
| F2 | — | Renombrar item |

### Estado desconectado
Escucha `window.addEventListener('ftp-disconnected')`. Si el `detail` coincide con `connId`, muestra overlay "Desconectado".

---

## AiPanel.vue

Asistente IA en sidebar. Soporta:
- **Claude** (Anthropic): claude-3-5-sonnet, claude-3-haiku, claude-opus-4
- **OpenAI**: gpt-4o, gpt-4o-mini, gpt-3.5-turbo
- **Gemini**: gemini-2.0-flash, gemini-1.5-flash-latest, gemini-1.5-pro-latest

Llama a `ai_chat(provider, model, api_key, messages[])` en Rust. Las API keys se guardan en `localStorage`.

---

## PreferencesModal.vue

Modal de preferencias. Al guardar, emite:
```javascript
window.dispatchEvent(new CustomEvent('prefs-changed', { detail: { fontFamily, fontSize, lineHeight, tabSize, wordWrap, autoSave } }))
```

Los valores se persisten en `localStorage`.

---

## useEditorStore.ts

Estado global reactivo sin librería externa.

```typescript
type TabType = 'code' | 'image' | 'database' | 'ftp'

interface Tab {
  path: string        // único identificador
  name: string
  content: string
  modified: boolean
  language: string    // detectado por nombre/extensión
  type: TabType
  // Para database tabs:
  connId?, tableName?, driver?
  // Para ftp tabs:
  ftpConnId?, ftpConnName?, ftpProtocol?
}
```

`detectLanguage(filename)` maneja:
- Mapa exacto de dotfiles (`.htaccess`, `.env`, `.prettierrc`, etc.)
- Mapa de extensiones (ts, js, vue, php, go, html, css, json, yaml, toml, xml, rs, py, sql, sh...)
