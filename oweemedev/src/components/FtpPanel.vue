<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useEditorStore } from '../composables/useEditorStore'

const store = useEditorStore()
const emit = defineEmits<{ 'open-conn': [connId: string, connName: string, protocol: string] }>()

interface FtpConn {
  id: string
  name: string
  protocol: 'sftp' | 'ftp'
  host: string
  port: number
  user: string
  pass: string
  root: string
  authType: 'password' | 'key'
}

const connections = ref<FtpConn[]>([])
const connectedIds = ref<Set<string>>(new Set())
const connectingId = ref<string | null>(null)
const errorMsg = ref('')
const showForm = ref(false)
const editingConn = ref<FtpConn | null>(null)

const form = ref<Omit<FtpConn, 'id'>>({
  name: '', protocol: 'sftp', host: '', port: 22,
  user: '', pass: '', root: '/', authType: 'password',
})

function load() {
  try { connections.value = JSON.parse(localStorage.getItem('ftp_connections') ?? '[]') } catch { connections.value = [] }
}

function save() { localStorage.setItem('ftp_connections', JSON.stringify(connections.value)) }

function openAddForm() {
  editingConn.value = null
  form.value = { name: '', protocol: 'sftp', host: '', port: 22, user: '', pass: '', root: '/', authType: 'password' }
  showForm.value = true
  errorMsg.value = ''
}

function openEditForm(c: FtpConn) {
  editingConn.value = c
  form.value = { ...c }
  showForm.value = true
  errorMsg.value = ''
}

function onProtocolChange() {
  form.value.port = form.value.protocol === 'sftp' ? 22 : 21
}

function cancelForm() { showForm.value = false; editingConn.value = null }

function submitForm() {
  if (!form.value.host || !form.value.user) { errorMsg.value = 'Host y usuario son requeridos'; return }
  if (editingConn.value) {
    Object.assign(editingConn.value, form.value)
  } else {
    connections.value.push({ ...form.value, id: crypto.randomUUID() })
  }
  save()
  showForm.value = false
  editingConn.value = null
}

function removeConn(id: string) {
  if (connectedIds.value.has(id)) invoke('remote_disconnect', { id })
  connectedIds.value.delete(id)
  connections.value = connections.value.filter(c => c.id !== id)
  save()
}

async function toggleConnect(c: FtpConn) {
  errorMsg.value = ''
  if (connectedIds.value.has(c.id)) {
    await invoke('remote_disconnect', { id: c.id })
    connectedIds.value.delete(c.id)
    connectedIds.value = new Set(connectedIds.value)
    window.dispatchEvent(new CustomEvent('ftp-disconnected', { detail: c.id }))
    return
  }
  connectingId.value = c.id
  try {
    await invoke('remote_connect', {
      id: c.id, protocol: c.protocol, host: c.host,
      port: c.port, user: c.user, pass: c.pass, root: c.root,
    })
    connectedIds.value.add(c.id)
    connectedIds.value = new Set(connectedIds.value)
    emit('open-conn', c.id, c.name, c.protocol)
    store.openFtpConn(c.id, c.name, c.protocol)
  } catch (e: any) {
    errorMsg.value = String(e)
  } finally {
    connectingId.value = null
  }
}

function openConn(c: FtpConn) {
  if (!connectedIds.value.has(c.id)) return
  store.openFtpConn(c.id, c.name, c.protocol)
}

onMounted(load)
</script>

<template>
  <div class="ftp-panel">
    <div class="ftp-header">
      <span class="ftp-title">FTP / SFTP</span>
      <button class="ftp-add-btn" @click="openAddForm" title="Nueva conexión">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 2a.5.5 0 01.5.5v5h5a.5.5 0 010 1h-5v5a.5.5 0 01-1 0v-5h-5a.5.5 0 010-1h5v-5A.5.5 0 018 2z"/>
        </svg>
      </button>
    </div>

    <!-- Error -->
    <div v-if="errorMsg" class="ftp-error">{{ errorMsg }}</div>

    <!-- Connection list -->
    <div class="ftp-list">
      <div v-if="connections.length === 0" class="ftp-empty">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".3">
          <path d="M9 3H5a2 2 0 00-2 2v4m6-6h10a2 2 0 012 2v4M9 3v18m0 0h10a2 2 0 002-2V9M9 21H5a2 2 0 01-2-2V9m0 0h18"/>
        </svg>
        <span>Sin conexiones</span>
        <button class="ftp-new-big" @click="openAddForm">+ Nueva conexión</button>
      </div>

      <div
        v-for="c in connections" :key="c.id"
        class="ftp-conn-row"
        :class="{ connected: connectedIds.has(c.id) }"
        @click="openConn(c)"
      >
        <div class="ftp-conn-icon">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 3H5a2 2 0 00-2 2v4m6-6h10a2 2 0 012 2v4M9 3v18m0 0h10a2 2 0 002-2V9M9 21H5a2 2 0 01-2-2V9m0 0h18"/>
          </svg>
        </div>
        <div class="ftp-conn-info">
          <div class="ftp-conn-name">{{ c.name }}</div>
          <div class="ftp-conn-host">{{ c.protocol.toUpperCase() }} · {{ c.user }}@{{ c.host }}</div>
        </div>
        <div class="ftp-conn-actions">
          <button
            class="ftp-action-btn"
            :class="{ connecting: connectingId === c.id, connected: connectedIds.has(c.id) }"
            @click.stop="toggleConnect(c)"
            :title="connectedIds.has(c.id) ? 'Desconectar' : 'Conectar'"
            :disabled="connectingId === c.id"
          >
            <span v-if="connectingId === c.id" class="conn-spin" />
            <svg v-else-if="connectedIds.has(c.id)" width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M5.5 2a3.5 3.5 0 100 7h5a3.5 3.5 0 000-7h-5zM8 3.5a2 2 0 110 4 2 2 0 010-4z"/>
              <path d="M3 8.5a4.5 4.5 0 009 0h-1a3.5 3.5 0 01-7 0H3z"/>
            </svg>
            <svg v-else width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M6.354 5.5H4a3 3 0 000 6h3a3 3 0 002.83-2H9c-.001 0-.002.001-.003.002A2 2 0 017 9.5H4a2 2 0 110-4h1.535c.218-.376.495-.714.82-1z"/>
              <path d="M9 5.5a3 3 0 012.83 2h.174c-.02.17-.03.342-.003.5H12a2 2 0 110 4H9.5a2 2 0 01-1.96-1.601A3 3 0 018.17 9.5H7a3 3 0 012-3z"/>
            </svg>
          </button>
          <button class="ftp-action-btn" @click.stop="openEditForm(c)" title="Editar">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M12.854.146a.5.5 0 00-.707 0L10.5 1.793 14.207 5.5l1.647-1.646a.5.5 0 000-.708l-3-3zm.646 6.061L9.793 2.5 3.293 9H3.5a.5.5 0 01.5.5v.5h.5a.5.5 0 01.5.5v.5h.5a.5.5 0 01.5.5v.5h.5a.5.5 0 01.5.5v.207l6.5-6.5zm-7.468 7.468A.5.5 0 015.5 14H5v-.5a.5.5 0 01-.5-.5H4v-.5a.5.5 0 01-.5-.5H3v-.5a.5.5 0 01-.5-.5H2.5v-.5a.5.5 0 01.646-.473l.853.214 1.832-1.832 3.709 3.709-1.832 1.832.214.853z"/>
            </svg>
          </button>
          <button class="ftp-action-btn ftp-action-del" @click.stop="removeConn(c.id)" title="Eliminar">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M5.5 5.5A.5.5 0 016 6v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm2.5 0a.5.5 0 01.5.5v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm3 .5a.5.5 0 00-1 0v6a.5.5 0 001 0V6z"/>
              <path fill-rule="evenodd" d="M14.5 3a1 1 0 01-1 1H13v9a2 2 0 01-2 2H5a2 2 0 01-2-2V4h-.5a1 1 0 010-2h4a1 1 0 011-1h2a1 1 0 011 1h4a1 1 0 011 1zM4.118 4L4 4.059V13a1 1 0 001 1h6a1 1 0 001-1V4.059L11.882 4H4.118zM2.5 3a.5.5 0 000 1h11a.5.5 0 000-1h-11z"/>
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- Add/Edit form overlay -->
    <div v-if="showForm" class="ftp-form-overlay" @click.self="cancelForm">
      <div class="ftp-form">
        <div class="ftp-form-title">{{ editingConn ? 'Editar' : 'Nueva' }} Conexión</div>

        <label>Nombre</label>
        <input v-model="form.name" class="ftp-input" placeholder="Mi servidor" />

        <label>Protocolo</label>
        <div class="ftp-proto-group">
          <button class="ftp-proto-btn" :class="{ active: form.protocol === 'sftp' }"
            @click="form.protocol = 'sftp'; onProtocolChange()">SFTP</button>
          <button class="ftp-proto-btn" :class="{ active: form.protocol === 'ftp' }"
            @click="form.protocol = 'ftp'; onProtocolChange()">FTP</button>
        </div>

        <div class="ftp-row2">
          <div>
            <label>Host</label>
            <input v-model="form.host" class="ftp-input" placeholder="ftp.empresa.com" />
          </div>
          <div style="width:80px">
            <label>Puerto</label>
            <input v-model.number="form.port" class="ftp-input" type="number" />
          </div>
        </div>

        <label>Usuario</label>
        <input v-model="form.user" class="ftp-input" placeholder="oweeme" />

        <div v-if="form.protocol === 'sftp'">
          <label>Autenticación</label>
          <div class="ftp-proto-group">
            <button class="ftp-proto-btn" :class="{ active: form.authType === 'password' }"
              @click="form.authType = 'password'">Contraseña</button>
            <button class="ftp-proto-btn" :class="{ active: form.authType === 'key' }"
              @click="form.authType = 'key'">SSH Key</button>
          </div>
        </div>

        <label>{{ form.authType === 'key' ? 'Passphrase (opcional)' : 'Contraseña' }}</label>
        <input v-model="form.pass" class="ftp-input" type="password"
          :placeholder="form.authType === 'key' ? '(usa ssh-agent si vacío)' : '••••••••'" />

        <label>Ruta remota inicial</label>
        <input v-model="form.root" class="ftp-input" placeholder="/public" />

        <div v-if="errorMsg" class="ftp-form-error">{{ errorMsg }}</div>

        <div class="ftp-form-footer">
          <button class="ftp-cancel-btn" @click="cancelForm">Cancelar</button>
          <button class="ftp-save-btn" @click="submitForm">
            {{ editingConn ? 'Guardar' : 'Crear' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ftp-panel {
  display: flex; flex-direction: column; height: 100%;
  font-family: var(--font-ui); position: relative;
}

.ftp-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 12px; height: 36px; flex-shrink: 0;
  border-bottom: 1px solid var(--border);
}
.ftp-title {
  font-size: 10px; font-weight: 700; letter-spacing: 1px;
  color: var(--fg-muted); text-transform: uppercase;
}
.ftp-add-btn {
  background: none; border: none; color: var(--fg-muted);
  cursor: pointer; padding: 3px; border-radius: 4px;
  display: flex; align-items: center; transition: color 0.12s;
}
.ftp-add-btn:hover { color: var(--fg); }

.ftp-error {
  margin: 8px 12px; padding: 8px 10px; background: rgba(248,81,73,0.12);
  border: 1px solid rgba(248,81,73,0.3); border-radius: 6px;
  font-size: 11px; color: #f85149; line-height: 1.5;
}

.ftp-list { flex: 1; overflow-y: auto; padding: 6px 0; }
.ftp-list::-webkit-scrollbar { width: 4px; }
.ftp-list::-webkit-scrollbar-thumb { background: var(--bg-active); border-radius: 2px; }

.ftp-empty {
  display: flex; flex-direction: column; align-items: center; gap: 10px;
  padding: 32px 16px; color: var(--fg-muted); font-size: 12px;
}
.ftp-new-big {
  background: rgba(46,158,135,0.12); border: 1px solid rgba(46,158,135,0.3);
  border-radius: 6px; color: var(--accent); font-size: 12px;
  padding: 6px 14px; cursor: pointer; transition: all 0.12s;
}
.ftp-new-big:hover { background: rgba(46,158,135,0.2); }

.ftp-conn-row {
  display: flex; align-items: center; gap: 8px;
  padding: 7px 12px; cursor: pointer;
  transition: background 0.1s; border-radius: 4px; margin: 1px 4px;
}
.ftp-conn-row:hover { background: var(--bg-hover); }
.ftp-conn-row.connected .ftp-conn-icon { color: var(--accent); }
.ftp-conn-icon { color: var(--fg-muted); flex-shrink: 0; }
.ftp-conn-info { flex: 1; min-width: 0; }
.ftp-conn-name { font-size: 12.5px; font-weight: 600; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.ftp-conn-host { font-size: 10.5px; color: var(--fg-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 1px; }

.ftp-conn-actions { display: flex; gap: 2px; flex-shrink: 0; }
.ftp-action-btn {
  width: 22px; height: 22px; background: none; border: none;
  border-radius: 4px; color: var(--fg-muted); cursor: pointer;
  display: flex; align-items: center; justify-content: center; transition: all 0.1s;
}
.ftp-action-btn:hover { background: var(--bg-active); color: var(--fg); }
.ftp-action-btn.connected { color: var(--accent); }
.ftp-action-btn.connecting { opacity: 0.5; cursor: default; }
.ftp-action-del:hover { color: #f85149 !important; }
.conn-spin {
  width: 10px; height: 10px; border: 2px solid rgba(46,158,135,0.3);
  border-top-color: var(--accent); border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* Form overlay */
.ftp-form-overlay {
  position: absolute; inset: 0; z-index: 100;
  background: rgba(0,0,0,0.7); display: flex; align-items: flex-start;
  padding: 16px 10px; overflow-y: auto;
}
.ftp-form {
  background: var(--bg-panel); border: 1px solid var(--border);
  border-radius: 10px; padding: 16px; width: 100%;
  display: flex; flex-direction: column; gap: 8px;
}
.ftp-form-title { font-size: 13px; font-weight: 700; color: var(--fg-bright); margin-bottom: 4px; }
.ftp-form label { font-size: 11px; color: var(--fg-dim); }
.ftp-input {
  width: 100%; background: var(--bg-mid); border: 1px solid var(--border);
  border-radius: 6px; color: var(--fg); font-size: 12px;
  padding: 5px 8px; outline: none; box-sizing: border-box;
  transition: border-color 0.12s;
}
.ftp-input:focus { border-color: var(--accent); }
.ftp-row2 { display: flex; gap: 8px; }
.ftp-row2 > div { flex: 1; display: flex; flex-direction: column; gap: 4px; }

.ftp-proto-group { display: flex; gap: 4px; }
.ftp-proto-btn {
  flex: 1; background: var(--bg-mid); border: 1px solid var(--border);
  border-radius: 5px; color: var(--fg-dim); font-size: 11px;
  padding: 4px; cursor: pointer; transition: all 0.12s;
}
.ftp-proto-btn.active {
  background: rgba(46,158,135,0.15); border-color: var(--accent);
  color: var(--accent); font-weight: 600;
}
.ftp-form-error {
  font-size: 11px; color: #f85149; padding: 6px 8px;
  background: rgba(248,81,73,0.1); border-radius: 5px;
}
.ftp-form-footer { display: flex; gap: 8px; margin-top: 4px; }
.ftp-cancel-btn {
  flex: 1; background: none; border: 1px solid var(--border);
  border-radius: 6px; color: var(--fg-muted); font-size: 12px;
  padding: 6px; cursor: pointer;
}
.ftp-cancel-btn:hover { border-color: var(--fg-muted); color: var(--fg); }
.ftp-save-btn {
  flex: 2; background: var(--accent); border: none; border-radius: 6px;
  color: #fff; font-size: 12px; font-weight: 600;
  padding: 6px; cursor: pointer; transition: opacity 0.12s;
}
.ftp-save-btn:hover { opacity: 0.85; }
</style>
