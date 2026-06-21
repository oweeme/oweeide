# Arquitectura

## Stack tecnológico

### Frontend (oweemedev/src/)

**Vue 3 + Composition API** — toda la UI está construida con `<script setup>` y Composition API. No se usa Options API.

**CodeMirror 6** — editor de código con:
- Compartments para actualización en vivo de configuraciones (font, tabSize, wordWrap)
- `@codemirror/lang-*` para lenguajes nativos (JS, TS, HTML, CSS, Rust, Python, Go, PHP, Vue, Markdown)
- `@codemirror/legacy-modes` para lenguajes via StreamLanguage (JSON, YAML, TOML, XML, Shell, Ruby, etc.)
- `@codemirror/lang-sql` con dialectos MySQL/PostgreSQL/SQLite y autocomplete de schema

**xterm.js** con `@xterm/addon-fit` y `@xterm/addon-web-links` — terminal en el panel inferior.

**Estado global** — `useEditorStore.ts` usa `reactive()` de Vue (sin Pinia, sin Vuex). Expone:
- `state.tabs[]` — tabs abiertos (code, image, database, ftp)
- `state.activeTabPath` — tab activo
- `state.rootPath` — carpeta raíz del proyecto abierto
- `state.cursorLine/cursorCol` — posición del cursor

### Backend (oweemedev/src-tauri/src/lib.rs)

Todos los comandos Tauri están en un único archivo `lib.rs`.

**Comandos de archivos:**
- `open_file(path)` → `String`
- `save_file(path, content)`
- `list_dir(path, show_hidden?)` → `Vec<DirEntry>`
- `create_file(path)`, `create_dir(path)`, `delete_path(path, is_dir)`, `rename_path(old, new)`
- `search_files(root, query)` → `Vec<SearchResult>`

**Comandos de terminal (PTY):**
- `pty_create(cols, rows)` → `String` (id)
- `pty_write(id, data)`
- `pty_read(id)` → `String`
- `pty_resize(id, cols, rows)`
- `pty_destroy(id)`

**Comandos de base de datos:**
- `db_connect(id, driver, host, port, user, pass, database)`
- `db_disconnect(id)`
- `db_list_tables(id)` → `Vec<TableInfo>`
- `db_query_table(id, table, page, page_size)` → `QueryResult`
- `db_execute(id, sql)` → `QueryResult`

**Comandos FTP/SFTP:**
- `remote_connect(id, protocol, host, port, user, pass, root)`
- `remote_disconnect(id)`
- `remote_list_dir(id, path)` → `Vec<RemoteEntry>`
- `remote_read_file(id, path)` → `String`
- `remote_write_file(id, path, content)`
- `remote_delete(id, path, is_dir)`
- `remote_mkdir(id, path)`
- `remote_rename(id, old_path, new_path)`

**Comando IA:**
- `ai_chat(provider, model, api_key, messages[])` → `String`

## Flujo de datos

```
Usuario → Vue component
  → invoke('comando_tauri') via @tauri-apps/api/core
  → lib.rs comando (Rust)
  → sistema de archivos / red / DB
  → resultado → Vue reactivo → UI actualizada
```

## Estado de conexiones (singleton en memoria Rust)

```rust
static DB_CONNS: Lazy<Mutex<HashMap<String, Arc<DbConn>>>>
static REMOTE:   Lazy<Mutex<HashMap<String, Arc<RemoteConn>>>>
static PTY_MAP:  Lazy<Mutex<HashMap<String, Arc<PtySession>>>>
```

Cada conexión se identifica con un `id` UUID generado en el frontend (`crypto.randomUUID()`).

## Operaciones bloqueantes

SSH2 y suppaftp son síncronas. Se envuelven en `tokio::task::spawn_blocking` para no bloquear el runtime async de Tauri:

```rust
let result = tokio::task::spawn_blocking(move || {
    // operación bloqueante aquí
}).await??;
```
