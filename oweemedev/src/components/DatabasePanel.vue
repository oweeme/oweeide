<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'

interface Connection {
  id: string
  name: string
  driver: 'mysql' | 'postgres' | 'sqlite'
  host: string
  port: number
  user: string
  password: string
  database: string
  filepath: string
}

const emit = defineEmits<{
  'open-table': [connId: string, tableName: string, connName: string, driver: string]
}>()

const connections = ref<Connection[]>(
  JSON.parse(localStorage.getItem('db_connections') || '[]')
)
const activeConnId = ref<string | null>(null)
const tables = ref<string[]>([])
const tableFilter = ref('')
const loading = ref(false)
const connError = ref('')
const showAddForm = ref(false)

const newConn = ref<Connection>({
  id: '', name: '', driver: 'mysql',
  host: 'localhost', port: 3306,
  user: 'root', password: '', database: '',
  filepath: '',
})

function saveConnections() {
  localStorage.setItem('db_connections', JSON.stringify(connections.value))
}

function buildUrl(c: Connection): string {
  if (c.driver === 'sqlite') return `sqlite://${c.filepath}`
  const scheme = c.driver === 'postgres' ? 'postgres' : 'mysql'
  const pass = encodeURIComponent(c.password)
  return `${scheme}://${c.user}:${pass}@${c.host}:${c.port}/${c.database}`
}

async function connect(conn: Connection) {
  loading.value = true
  connError.value = ''
  tables.value = []
  try {
    await invoke('db_connect', { id: conn.id, driver: conn.driver, url: buildUrl(conn) })
    activeConnId.value = conn.id
    tables.value = await invoke<string[]>('db_list_tables', { id: conn.id })
  } catch (e: any) {
    connError.value = String(e)
    activeConnId.value = null
  } finally {
    loading.value = false
  }
}

async function disconnect() {
  if (!activeConnId.value) return
  await invoke('db_disconnect', { id: activeConnId.value }).catch(() => {})
  activeConnId.value = null
  tables.value = []
}

function openTable(table: string) {
  const conn = connections.value.find(c => c.id === activeConnId.value)
  if (!conn) return
  emit('open-table', conn.id, table, conn.name, conn.driver)
}

function removeConnection(id: string) {
  if (activeConnId.value === id) disconnect()
  connections.value = connections.value.filter(c => c.id !== id)
  saveConnections()
}

async function addConnection() {
  const c = { ...newConn.value, id: `db-${Date.now()}` }
  connections.value.push(c)
  saveConnections()
  showAddForm.value = false
  resetForm()
  await connect(c)
}

function resetForm() {
  newConn.value = { id: '', name: '', driver: 'mysql', host: 'localhost', port: 3306, user: 'root', password: '', database: '', filepath: '' }
}

async function browseSqliteFile() {
  const selected = await openDialog({
    filters: [{ name: 'SQLite Database', extensions: ['db', 'sqlite', 'sqlite3'] }],
    multiple: false,
  })
  if (typeof selected === 'string') newConn.value.filepath = selected
}

function onDriverChange() {
  if (newConn.value.driver === 'postgres') newConn.value.port = 5432
  else if (newConn.value.driver === 'mysql') newConn.value.port = 3306
}

const filteredTables = computed(() =>
  tableFilter.value
    ? tables.value.filter(t => t.toLowerCase().includes(tableFilter.value.toLowerCase()))
    : tables.value
)

async function refreshTables() {
  if (!activeConnId.value) return
  loading.value = true
  try {
    tables.value = await invoke<string[]>('db_list_tables', { id: activeConnId.value })
  } catch (e: any) { connError.value = String(e) }
  finally { loading.value = false }
}
</script>

<template>
  <div class="db-panel">
    <!-- Header -->
    <div class="db-header">
      <div class="db-title">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <ellipse cx="12" cy="5" rx="9" ry="3"/>
          <path d="M3 5v4c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
          <path d="M3 9v4c0 1.66 4.03 3 9 3s9-1.34 9-3V9"/>
          <path d="M3 13v4c0 1.66 4.03 3 9 3s9-1.34 9-3v-4"/>
        </svg>
        DATABASE
      </div>
      <div class="db-header-btns">
        <button v-if="activeConnId" class="db-icon-btn" @click="refreshTables" title="Refrescar tablas">
          <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2v1z"/>
            <path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466z"/>
          </svg>
        </button>
        <button class="db-icon-btn" @click="showAddForm = !showAddForm" title="Nueva conexión">
          {{ showAddForm ? '✕' : '+' }}
        </button>
      </div>
    </div>

    <!-- Add connection form -->
    <div v-if="showAddForm" class="db-form">
      <div class="form-field">
        <label>Nombre de la conexión</label>
        <input v-model="newConn.name" placeholder="Mi base de datos" class="form-input" />
      </div>
      <div class="form-field">
        <label>Motor</label>
        <div class="driver-tabs">
          <button v-for="d in ['mysql', 'postgres', 'sqlite']" :key="d"
            class="driver-tab" :class="{ active: newConn.driver === d }"
            @click="newConn.driver = d as any; onDriverChange()">{{ d }}</button>
        </div>
      </div>
      <template v-if="newConn.driver !== 'sqlite'">
        <div class="form-row">
          <div class="form-field" style="flex:1">
            <label>Host</label>
            <input v-model="newConn.host" class="form-input" placeholder="localhost" />
          </div>
          <div class="form-field" style="width:68px">
            <label>Puerto</label>
            <input v-model.number="newConn.port" type="number" class="form-input" />
          </div>
        </div>
        <div class="form-field">
          <label>Usuario</label>
          <input v-model="newConn.user" class="form-input" />
        </div>
        <div class="form-field">
          <label>Contraseña</label>
          <input v-model="newConn.password" type="password" class="form-input" />
        </div>
        <div class="form-field">
          <label>Base de datos</label>
          <input v-model="newConn.database" class="form-input" />
        </div>
      </template>
      <template v-else>
        <div class="form-field">
          <label>Archivo SQLite</label>
          <div class="form-file-row">
            <input v-model="newConn.filepath" class="form-input" placeholder="/ruta/archivo.db" style="flex:1" />
            <button class="form-browse-btn" @click="browseSqliteFile">…</button>
          </div>
        </div>
      </template>
      <div class="form-actions">
        <button class="form-cancel" @click="showAddForm = false; resetForm()">Cancelar</button>
        <button class="form-connect" @click="addConnection" :disabled="!newConn.name">Conectar</button>
      </div>
    </div>

    <!-- Error bar -->
    <div v-if="connError" class="db-error-bar">
      <span>{{ connError }}</span>
      <button @click="connError = ''">×</button>
    </div>

    <!-- Connection list -->
    <div class="db-conn-list">
      <div v-if="connections.length === 0 && !showAddForm" class="db-empty">
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" opacity=".25">
          <ellipse cx="12" cy="5" rx="9" ry="3"/>
          <path d="M3 5v4c0 1.66 4.03 3 9 3s9-1.34 9-3V5"/>
          <path d="M3 9v4c0 1.66 4.03 3 9 3s9-1.34 9-3V9"/>
          <path d="M3 13v4c0 1.66 4.03 3 9 3s9-1.34 9-3v-4"/>
        </svg>
        <span>Haz clic en + para conectar</span>
      </div>

      <div v-for="conn in connections" :key="conn.id"
        class="db-conn-item" :class="{ active: conn.id === activeConnId }">
        <span class="conn-dot" :class="conn.id === activeConnId ? 'dot-on' : 'dot-off'" />
        <div class="conn-info" @click="connect(conn)" style="cursor:pointer">
          <span class="conn-name">{{ conn.name }}</span>
          <span class="conn-meta">{{ conn.driver }} · {{ conn.driver === 'sqlite' ? conn.filepath.split('/').pop() : conn.database }}</span>
        </div>
        <button class="conn-btn" :title="conn.id === activeConnId ? 'Desconectar' : 'Conectar'"
          @click="conn.id === activeConnId ? disconnect() : connect(conn)">
          {{ conn.id === activeConnId ? '⏏' : '▶' }}
        </button>
        <button class="conn-btn conn-del" @click="removeConnection(conn.id)" title="Eliminar">×</button>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="db-loading"><span class="db-spinner" /> Conectando…</div>

    <!-- Table tree (when connected) -->
    <template v-if="activeConnId && !loading">
      <div class="section-label">
        TABLAS ({{ filteredTables.length }})
      </div>
      <div class="table-filter-wrap">
        <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" style="color:var(--fg-muted);flex-shrink:0">
          <path d="M11.742 10.344a6.5 6.5 0 10-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 001.415-1.414l-3.85-3.85a1.007 1.007 0 00-.115-.099zm-5.242 1.656a5.5 5.5 0 110-11 5.5 5.5 0 010 11z"/>
        </svg>
        <input v-model="tableFilter" class="table-filter" placeholder="Filtrar…" />
      </div>
      <div class="table-list">
        <div v-for="t in filteredTables" :key="t" class="table-item" @click="openTable(t)">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" style="flex-shrink:0;color:var(--fg-muted)">
            <rect x="3" y="3" width="18" height="18" rx="1"/>
            <path d="M3 9h18M3 15h18M9 3v18"/>
          </svg>
          {{ t }}
        </div>
        <div v-if="filteredTables.length === 0" class="table-none">Sin resultados</div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.db-panel {
  display: flex; flex-direction: column;
  height: 100%; background: var(--bg-dark);
  font-family: var(--font-ui); overflow: hidden;
}
.db-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 8px 12px; border-bottom: 1px solid var(--border);
  background: var(--bg-darkest); flex-shrink: 0;
}
.db-title {
  display: flex; align-items: center; gap: 6px;
  font-size: 10.5px; font-weight: 700; letter-spacing: 0.8px; color: var(--fg-muted);
}
.db-header-btns { display: flex; gap: 4px; }
.db-icon-btn {
  width: 22px; height: 22px; border-radius: 5px;
  background: none; border: 1px solid var(--border);
  color: var(--fg-muted); font-size: 14px; cursor: pointer;
  display: flex; align-items: center; justify-content: center; transition: all 0.12s;
}
.db-icon-btn:hover { background: var(--bg-hover); color: var(--fg); border-color: var(--accent); }

.db-form {
  background: var(--bg-mid); border-bottom: 1px solid var(--border);
  padding: 12px; flex-shrink: 0;
}
.form-field { margin-bottom: 7px; }
.form-field label { display: block; font-size: 10px; color: var(--fg-muted); margin-bottom: 3px; }
.form-input {
  width: 100%; background: var(--bg-darkest); border: 1px solid var(--border);
  border-radius: 4px; color: var(--fg); font-size: 12px; padding: 4px 7px;
  outline: none; font-family: var(--font-mono);
}
.form-input:focus { border-color: var(--accent); }
.form-row { display: flex; gap: 6px; }
.form-file-row { display: flex; gap: 5px; }
.form-browse-btn {
  background: var(--bg-hover); border: 1px solid var(--border);
  border-radius: 4px; color: var(--fg-dim); padding: 0 8px;
  cursor: pointer; font-size: 13px; flex-shrink: 0;
}
.driver-tabs { display: flex; gap: 4px; }
.driver-tab {
  flex: 1; padding: 4px 0; border: 1px solid var(--border); border-radius: 4px;
  background: none; color: var(--fg-dim); font-size: 11px; cursor: pointer; text-align: center;
}
.driver-tab.active { background: rgba(46,158,135,0.15); border-color: var(--accent); color: var(--accent); font-weight: 600; }
.form-actions { display: flex; gap: 6px; margin-top: 8px; }
.form-cancel {
  flex: 1; background: none; border: 1px solid var(--border);
  color: var(--fg-muted); border-radius: 5px; font-size: 11.5px; padding: 5px; cursor: pointer;
}
.form-connect {
  flex: 2; background: var(--accent); border: none; border-radius: 5px;
  color: #fff; font-size: 11.5px; font-weight: 600; padding: 5px; cursor: pointer;
}
.form-connect:disabled { opacity: 0.4; cursor: default; }

.db-error-bar {
  display: flex; align-items: flex-start; justify-content: space-between; gap: 6px;
  background: rgba(248,81,73,0.1); border-bottom: 1px solid rgba(248,81,73,0.3);
  padding: 7px 10px; font-size: 10.5px; color: #f85149; flex-shrink: 0; line-height: 1.4;
}
.db-error-bar button {
  background: none; border: none; color: #f85149; cursor: pointer; font-size: 14px; flex-shrink: 0;
}

.db-conn-list { flex-shrink: 0; }
.db-empty {
  display: flex; flex-direction: column; align-items: center;
  gap: 6px; padding: 24px 0; color: var(--fg-muted); font-size: 11px;
}
.db-conn-item {
  display: flex; align-items: center; gap: 6px;
  padding: 6px 10px; border-bottom: 1px solid rgba(255,255,255,0.04); transition: background 0.1s;
}
.db-conn-item:hover { background: var(--bg-hover); }
.db-conn-item.active { background: rgba(46,158,135,0.07); }
.conn-dot { width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
.dot-on  { background: var(--accent); box-shadow: 0 0 4px rgba(46,158,135,0.6); }
.dot-off { background: var(--fg-muted); opacity: 0.4; }
.conn-info { flex: 1; min-width: 0; }
.conn-name { display: block; font-size: 12px; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.conn-meta { display: block; font-size: 10px; color: var(--fg-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.conn-btn {
  background: none; border: none; color: var(--fg-muted); font-size: 12px;
  cursor: pointer; padding: 2px 5px; border-radius: 3px; transition: all 0.1s; flex-shrink: 0;
}
.conn-btn:hover { color: var(--accent); background: var(--bg-hover); }
.conn-del:hover { color: var(--red); }

.db-loading {
  display: flex; align-items: center; gap: 7px;
  padding: 10px 14px; font-size: 11.5px; color: var(--fg-muted); flex-shrink: 0;
}
.db-spinner {
  width: 12px; height: 12px; border: 2px solid rgba(46,158,135,0.3);
  border-top-color: var(--accent); border-radius: 50%; animation: spin 0.7s linear infinite; display: inline-block;
}
@keyframes spin { to { transform: rotate(360deg); } }

.section-label {
  padding: 8px 12px 4px; font-size: 9.5px; font-weight: 700;
  letter-spacing: 0.8px; color: var(--fg-muted); flex-shrink: 0;
}
.table-filter-wrap {
  display: flex; align-items: center; gap: 7px;
  padding: 4px 10px 6px; flex-shrink: 0;
}
.table-filter {
  flex: 1; background: none; border: none;
  color: var(--fg); font-size: 12px; outline: none; font-family: var(--font-ui);
}
.table-filter::placeholder { color: var(--fg-muted); }
.table-list { flex: 1; overflow-y: auto; }
.table-list::-webkit-scrollbar { width: 3px; }
.table-list::-webkit-scrollbar-thumb { background: var(--bg-active); border-radius: 2px; }
.table-item {
  display: flex; align-items: center; gap: 7px;
  padding: 5px 14px; font-size: 12px; color: var(--fg-dim);
  cursor: pointer; transition: background 0.1s;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.table-item:hover { background: var(--bg-hover); color: var(--fg); }
.table-none { padding: 8px 14px; font-size: 11px; color: var(--fg-muted); }
</style>
