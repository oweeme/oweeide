<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useEditorStore } from '../composables/useEditorStore'

interface Props { connId: string; connName: string; protocol: string }
interface RemoteEntry { name: string; path: string; is_dir: boolean; size: number }
interface LocalEntry  { name: string; path: string; is_dir: boolean }

const props = defineProps<Props>()
const store = useEditorStore()

// ── Remote state ───────────────────────────────────────────────────────────────
const remotePath       = ref('/')
const remoteStack      = ref<string[]>([])
const remoteItems      = ref<RemoteEntry[]>([])
const remoteLoading    = ref(false)
const remoteError      = ref('')
const remoteSelected   = ref(new Set<string>())   // multi-select by path
const remotePreviewEntry = ref<RemoteEntry | null>(null)
const remoteContent    = ref<string | null>(null)
const remotePreview    = ref(false)
const isConnected      = ref(true)

// ── Local state ────────────────────────────────────────────────────────────────
const localPath    = ref(store.state.rootPath || '')
const localStack   = ref<string[]>([])
const localItems   = ref<LocalEntry[]>([])
const localLoading = ref(false)
// Multi-select: Set of paths
const localSelected = ref(new Set<string>())
// Local preview
const localPreviewEntry = ref<LocalEntry | null>(null)
const localContent      = ref<string | null>(null)
const localPreview      = ref(false)

// ── Upload progress ────────────────────────────────────────────────────────────
const uploadProgress = ref<{ done: number; total: number } | null>(null)

// ── Keyboard navigation ────────────────────────────────────────────────────────
const localFocusIdx  = ref(-1)
const remoteFocusIdx = ref(-1)
const localBodyEl    = ref<HTMLElement | null>(null)
const remoteBodyEl   = ref<HTMLElement | null>(null)

function scrollRowIntoView(el: HTMLElement | null, idx: number) {
  if (!el) return
  const row = el.querySelectorAll<HTMLElement>('.pane-row')[idx]
  row?.scrollIntoView({ block: 'nearest' })
}

function onLocalKeydown(e: KeyboardEvent) {
  if (renaming.value) return
  const len = localItems.value.length
  if (len === 0) return

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    localFocusIdx.value = Math.min(localFocusIdx.value + 1, len - 1)
    scrollRowIntoView(localBodyEl.value, localFocusIdx.value)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    localFocusIdx.value = Math.max(localFocusIdx.value - 1, 0)
    scrollRowIntoView(localBodyEl.value, localFocusIdx.value)
  } else if (e.key === ' ') {
    e.preventDefault()
    const entry = localItems.value[localFocusIdx.value]
    if (!entry) return
    const s = new Set(localSelected.value)
    if (s.has(entry.path)) s.delete(entry.path)
    else s.add(entry.path)
    localSelected.value = s
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const entry = localItems.value[localFocusIdx.value]
    if (!entry) return
    if (entry.is_dir) { localStack.value.push(localPath.value); loadLocal(entry.path) }
    else openLocalPreview(entry)
  } else if (e.key === 'Backspace' || e.key === 'ArrowLeft') {
    e.preventDefault()
    localBack()
  } else if (e.key === 'a' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    localSelected.value = new Set(localItems.value.map(e => e.path))
  } else if (e.key === 'Escape') {
    localSelected.value = new Set()
  } else if (e.key === 'u' || e.key === 'U') {
    e.preventDefault()
    uploadSelected()
  }
}

function onRemoteKeydown(e: KeyboardEvent) {
  if (renaming.value) return
  const len = remoteItems.value.length
  if (len === 0) return

  if (e.key === 'ArrowDown') {
    e.preventDefault()
    remoteFocusIdx.value = Math.min(remoteFocusIdx.value + 1, len - 1)
    scrollRowIntoView(remoteBodyEl.value, remoteFocusIdx.value)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    remoteFocusIdx.value = Math.max(remoteFocusIdx.value - 1, 0)
    scrollRowIntoView(remoteBodyEl.value, remoteFocusIdx.value)
  } else if (e.key === ' ') {
    e.preventDefault()
    const entry = remoteItems.value[remoteFocusIdx.value]
    if (!entry) return
    const s = new Set(remoteSelected.value)
    if (s.has(entry.path)) s.delete(entry.path)
    else s.add(entry.path)
    remoteSelected.value = s
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const entry = remoteItems.value[remoteFocusIdx.value]
    if (!entry) return
    if (entry.is_dir) { remoteStack.value.push(remotePath.value); loadRemote(entry.path) }
    else previewRemote(entry)
  } else if (e.key === 'Backspace' || e.key === 'ArrowLeft') {
    e.preventDefault()
    remoteBack()
  } else if (e.key === 'Escape') {
    remoteSelected.value = new Set()
  } else if ((e.key === 'a' || e.key === 'A') && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    remoteSelected.value = new Set(remoteItems.value.map(r => r.path))
  } else if (e.key === 'Delete') {
    e.preventDefault()
    deleteRemoteSelected()
  } else if (e.key === 'd' || e.key === 'D') {
    e.preventDefault()
    downloadSelected()
  } else if (e.key === 'F2') {
    e.preventDefault()
    const entry = remoteItems.value[remoteFocusIdx.value]
    if (entry) startRename(entry)
  }
}

// Reset focus index when items reload
watch(localItems, () => { localFocusIdx.value = localItems.value.length > 0 ? 0 : -1 })
watch(remoteItems, () => { remoteFocusIdx.value = remoteItems.value.length > 0 ? 0 : -1; remoteSelected.value = new Set() })

// ── UI state ───────────────────────────────────────────────────────────────────
const splitPos     = ref(50)
const isDragging   = ref(false)
const statusMsg    = ref('')
const statusType   = ref<'ok' | 'err' | ''>('')
const showNewDir   = ref<'remote' | null>(null)
const newDirName   = ref('')
const renaming     = ref<{ entry: RemoteEntry; name: string } | null>(null)
const transferring = ref(false)

// ── Splitter ──────────────────────────────────────────────────────────────────
function onSplitMousedown(e: MouseEvent) {
  isDragging.value = true
  const container = (e.currentTarget as HTMLElement).closest('.ftp-main') as HTMLElement
  const onMove = (ev: MouseEvent) => {
    const rect = container.getBoundingClientRect()
    splitPos.value = Math.max(22, Math.min(75, ((ev.clientX - rect.left) / rect.width) * 100))
  }
  const onUp = () => { isDragging.value = false; window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp) }
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

// ── Local navigation ───────────────────────────────────────────────────────────
async function loadLocal(path: string) {
  localLoading.value = true
  localSelected.value = new Set()
  localPreview.value = false
  try {
    localItems.value = await invoke<LocalEntry[]>('list_dir', { path, showHidden: true })
    localPath.value  = path
  } catch (e) { setStatus(String(e), 'err') }
  finally { localLoading.value = false }
}

function localClick(entry: LocalEntry, e: MouseEvent) {
  if (e.ctrlKey || e.metaKey) {
    // multi-select toggle
    const s = new Set(localSelected.value)
    if (s.has(entry.path)) s.delete(entry.path)
    else s.add(entry.path)
    localSelected.value = s
  } else if (entry.is_dir) {
    // navigate
    localStack.value.push(localPath.value)
    loadLocal(entry.path)
  } else {
    localSelected.value = new Set([entry.path])
  }
}

function localDblclick(entry: LocalEntry) {
  if (entry.is_dir) return
  openLocalPreview(entry)
}

function localBack()  { const p = localStack.value.pop(); if (p) loadLocal(p) }
function localHome()  { localStack.value = []; loadLocal(store.state.rootPath || localPath.value) }
function localUp()    {
  const parent = localPath.value.split('/').slice(0, -1).join('/') || '/'
  localStack.value.push(localPath.value)
  loadLocal(parent)
}

// ── Local preview ──────────────────────────────────────────────────────────────
async function openLocalPreview(entry: LocalEntry) {
  localPreviewEntry.value = entry
  localPreview.value = true
  localContent.value = null
  try {
    localContent.value = await invoke<string>('open_file', { path: entry.path })
  } catch (e: any) { localContent.value = `[Error al leer: ${e}]` }
}

async function saveLocal() {
  if (!localPreviewEntry.value || localContent.value === null) return
  try {
    await invoke('save_file', { path: localPreviewEntry.value.path, content: localContent.value })
    setStatus(`Guardado: ${localPreviewEntry.value.name}`, 'ok')
  } catch (e: any) { setStatus(String(e), 'err') }
}

// ── Remote navigation ──────────────────────────────────────────────────────────
async function loadRemote(path: string) {
  remoteLoading.value = true
  remoteError.value = ''
  remoteContent.value = null
  remotePreview.value = false
  remoteSelected.value = new Set()
  try {
    remoteItems.value = await invoke<RemoteEntry[]>('remote_list_dir', { id: props.connId, path })
    remotePath.value  = path
  } catch (e: any) { remoteError.value = String(e) }
  finally { remoteLoading.value = false }
}

function remoteClick(entry: RemoteEntry, e: MouseEvent) {
  if (entry.is_dir) {
    remoteStack.value.push(remotePath.value)
    loadRemote(entry.path)
    return
  }
  if (e.ctrlKey || e.metaKey) {
    const s = new Set(remoteSelected.value)
    if (s.has(entry.path)) s.delete(entry.path)
    else s.add(entry.path)
    remoteSelected.value = s
  } else {
    remoteSelected.value = new Set([entry.path])
    remotePreviewEntry.value = entry
  }
}

function remoteBack() { const p = remoteStack.value.pop(); if (p) loadRemote(p) }

const remoteBreadcrumbs = computed(() => {
  const parts = remotePath.value.split('/').filter(Boolean)
  const r = [{ label: '/', path: '/' }]
  let cur = ''
  for (const p of parts) { cur += '/' + p; r.push({ label: p, path: cur }) }
  return r
})

// ── Remote preview → open as editor tab ────────────────────────────────────────
async function previewRemote(entry: RemoteEntry) {
  if (entry.is_dir) return
  remoteSelected.value = new Set([entry.path])
  try {
    const content = await invoke<string>('remote_read_file', { id: props.connId, path: entry.path })
    store.openFtpFile(props.connId, entry.path, entry.name, content)
    setStatus(`Abierto: ${entry.name}`, 'ok')
  } catch (e: any) {
    // Fallback to inline preview for binary/unreadable files
    remotePreviewEntry.value = entry
    remotePreview.value = true
    remoteContent.value = `[Error al leer: ${e}]`
    setStatus(String(e), 'err')
  }
}

async function saveRemote() {
  if (!remotePreviewEntry.value || remoteContent.value === null) return
  try {
    await invoke('remote_write_file', { id: props.connId, path: remotePreviewEntry.value.path, content: remoteContent.value })
    setStatus(`Guardado en servidor: ${remotePreviewEntry.value.name}`, 'ok')
  } catch (e: any) { setStatus(String(e), 'err') }
}

// ── Multi-file/folder Upload ───────────────────────────────────────────────────
// Collect all files recursively from a local entry
async function collectFiles(entry: LocalEntry, remoteBase: string): Promise<{ localPath: string; remotePath: string }[]> {
  if (!entry.is_dir) {
    return [{ localPath: entry.path, remotePath: remoteBase + '/' + entry.name }]
  }
  const children = await invoke<LocalEntry[]>('list_dir', { path: entry.path, showHidden: true })
  const results: { localPath: string; remotePath: string }[] = []
  const subBase = remoteBase + '/' + entry.name
  for (const child of children) {
    results.push(...await collectFiles(child, subBase))
  }
  return results
}

async function uploadSelected() {
  if (localSelected.value.size === 0) { setStatus('Selecciona archivos o carpetas con Ctrl+click', 'err'); return }
  transferring.value = true
  uploadProgress.value = null

  const selectedEntries = localItems.value.filter(e => localSelected.value.has(e.path))

  // First collect all files to know the total
  let allFiles: { localPath: string; remotePath: string }[] = []
  const remoteBase = remotePath.value === '/' ? '' : remotePath.value
  for (const entry of selectedEntries) {
    try { allFiles.push(...await collectFiles(entry, remoteBase)) }
    catch (e: any) { setStatus(String(e), 'err') }
  }

  // Create remote directories for folder uploads
  const dirs = new Set<string>()
  for (const f of allFiles) {
    const parent = f.remotePath.split('/').slice(0, -1).join('/') || '/'
    dirs.add(parent)
  }
  for (const dir of [...dirs].sort()) {
    if (dir === '/' || dir === remoteBase) continue
    try { await invoke('remote_mkdir', { id: props.connId, path: dir }) } catch { /* exists */ }
  }

  uploadProgress.value = { done: 0, total: allFiles.length }
  let errors = 0
  for (const f of allFiles) {
    try {
      const content = await invoke<string>('open_file', { path: f.localPath })
      await invoke('remote_write_file', { id: props.connId, path: f.remotePath, content })
    } catch { errors++ }
    uploadProgress.value = { done: uploadProgress.value.done + 1, total: allFiles.length }
  }

  if (errors === 0) setStatus(`Subidos ${allFiles.length} archivo(s)`, 'ok')
  else setStatus(`${allFiles.length - errors} subidos, ${errors} error(es)`, 'err')

  uploadProgress.value = null
  transferring.value = false
  await loadRemote(remotePath.value)
}

async function downloadSelected() {
  const toDownload = remoteItems.value.filter(e => remoteSelected.value.has(e.path) && !e.is_dir)
  if (toDownload.length === 0) { setStatus('Selecciona archivos remotos para descargar', 'err'); return }
  transferring.value = true
  uploadProgress.value = { done: 0, total: toDownload.length }
  let errors = 0
  const base = localPath.value.endsWith('/') ? localPath.value : localPath.value + '/'
  for (const entry of toDownload) {
    try {
      const content = await invoke<string>('remote_read_file', { id: props.connId, path: entry.path })
      await invoke('save_file', { path: base + entry.name, content })
    } catch { errors++ }
    uploadProgress.value = { done: uploadProgress.value!.done + 1, total: toDownload.length }
  }
  uploadProgress.value = null
  if (errors === 0) setStatus(`Descargados ${toDownload.length} archivo(s)`, 'ok')
  else setStatus(`${toDownload.length - errors} descargados, ${errors} error(es)`, 'err')
  await loadLocal(localPath.value)
  transferring.value = false
}

// ── Remote actions ─────────────────────────────────────────────────────────────
async function remoteDelete(entry: RemoteEntry) {
  if (!confirm(`¿Eliminar "${entry.name}"?`)) return
  try {
    await invoke('remote_delete', { id: props.connId, path: entry.path, isDir: entry.is_dir })
    if (remotePreviewEntry.value?.path === entry.path) { remotePreviewEntry.value = null; remotePreview.value = false }
    setStatus(`Eliminado: ${entry.name}`, 'ok')
    await loadRemote(remotePath.value)
  } catch (e: any) { setStatus(String(e), 'err') }
}

async function deleteRemoteSelected() {
  const toDelete = remoteItems.value.filter(e => remoteSelected.value.has(e.path))
  if (toDelete.length === 0) return
  if (!confirm(`¿Eliminar ${toDelete.length} elemento(s)?`)) return
  let errors = 0
  for (const entry of toDelete) {
    try { await invoke('remote_delete', { id: props.connId, path: entry.path, isDir: entry.is_dir }) }
    catch { errors++ }
  }
  if (remotePreviewEntry.value && remoteSelected.value.has(remotePreviewEntry.value.path)) {
    remotePreviewEntry.value = null; remotePreview.value = false
  }
  if (errors === 0) setStatus(`Eliminados ${toDelete.length} elemento(s)`, 'ok')
  else setStatus(`${toDelete.length - errors} eliminados, ${errors} error(es)`, 'err')
  await loadRemote(remotePath.value)
}

function startRename(entry: RemoteEntry) { renaming.value = { entry, name: entry.name } }

async function confirmRename() {
  if (!renaming.value) return
  const { entry, name } = renaming.value
  if (name === entry.name) { renaming.value = null; return }
  const newPath = entry.path.split('/').slice(0, -1).join('/') + '/' + name
  try {
    await invoke('remote_rename', { id: props.connId, oldPath: entry.path, newPath })
    renaming.value = null; setStatus(`Renombrado → ${name}`, 'ok')
    await loadRemote(remotePath.value)
  } catch (e: any) { setStatus(String(e), 'err') }
}

async function createRemoteDir() {
  if (!newDirName.value.trim()) return
  const name = newDirName.value.trim(); newDirName.value = ''; showNewDir.value = null
  const newPath = (remotePath.value === '/' ? '' : remotePath.value) + '/' + name
  try {
    await invoke('remote_mkdir', { id: props.connId, path: newPath })
    setStatus(`Carpeta creada: ${name}`, 'ok'); await loadRemote(remotePath.value)
  } catch (e: any) { setStatus(String(e), 'err') }
}

// ── Helpers ────────────────────────────────────────────────────────────────────
function setStatus(msg: string, type: 'ok' | 'err') {
  statusMsg.value = msg; statusType.value = type
  setTimeout(() => { statusMsg.value = ''; statusType.value = '' }, 5000)
}

function formatSize(n: number) {
  if (n === 0) return ''
  if (n < 1024) return `${n} B`
  if (n < 1048576) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1048576).toFixed(1)} MB`
}

function fileColor(name: string) {
  const ext = name.split('.').pop()?.toLowerCase() ?? ''
  const m: Record<string, string> = {
    php:'#8892be', js:'#f7df1e', ts:'#3178c6', vue:'#42b883', html:'#e44d26',
    htm:'#e44d26', css:'#264de4', scss:'#cc6699', json:'#cbcb41', yaml:'#cbcb41',
    yml:'#cbcb41', sql:'#e38c00', py:'#3572a5', rs:'#ce422b', go:'#00add8',
    md:'#aaa', txt:'#888', jpg:'#ff79a8', jpeg:'#ff79a8', png:'#ff79a8',
    gif:'#ff79a8', svg:'#ffb86c', zip:'#bd93f9', tar:'#bd93f9', gz:'#bd93f9',
    htaccess:'#50fa7b', env:'#f1fa8c', xml:'#e37933',
  }
  return m[ext] ?? '#888'
}

function onFtpDisconnected(e: Event) {
  if ((e as CustomEvent).detail === props.connId) isConnected.value = false
}

onMounted(async () => {
  window.addEventListener('ftp-disconnected', onFtpDisconnected)
  if (store.state.rootPath) await loadLocal(store.state.rootPath)
  await loadRemote('/')
})

onBeforeUnmount(() => {
  window.removeEventListener('ftp-disconnected', onFtpDisconnected)
})

watch(() => props.connId, () => { isConnected.value = true; loadRemote('/') })
</script>

<template>
  <div class="ftp-view" :class="{ dragging: isDragging }">

    <!-- Disconnected overlay -->
    <div v-if="!isConnected" class="disconnected-overlay">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".4">
        <path d="M18.364 5.636a9 9 0 010 12.728M15.536 8.464a5 5 0 010 7.072M6.343 17.657a9 9 0 010-12.728M9.172 14.828a5 5 0 010-7.07"/>
        <line x1="2" y1="2" x2="22" y2="22"/>
      </svg>
      <div class="disc-title">Desconectado</div>
      <div class="disc-sub">La conexión "{{ connName }}" se ha cerrado.<br>Vuelve a conectar desde el panel FTP/SFTP.</div>
    </div>

    <!-- Top bar -->
    <div class="ftp-topbar">
      <div class="ftp-conn-badge">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M9 3H5a2 2 0 00-2 2v4m6-6h10a2 2 0 012 2v4M9 3v18m0 0h10a2 2 0 002-2V9M9 21H5a2 2 0 01-2-2V9m0 0h18"/>
        </svg>
        <span class="ftp-badge-name">{{ connName }}</span>
        <span class="ftp-badge-proto">{{ protocol.toUpperCase() }}</span>
      </div>

      <div class="ftp-transfer-actions">
        <!-- Upload progress bar -->
        <template v-if="uploadProgress">
          <div class="upload-progress">
            <div class="upload-progress-bar" :style="{ width: (uploadProgress.done / uploadProgress.total * 100) + '%' }" />
          </div>
          <span class="upload-count">{{ uploadProgress.done }}/{{ uploadProgress.total }}</span>
        </template>
        <template v-else>
          <button
            class="ftp-xfer-btn upload"
            @click="uploadSelected"
            :disabled="localSelected.size === 0 || transferring"
            :title="`Subir ${localSelected.size} elemento(s) seleccionado(s)`"
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M.5 9.9a.5.5 0 01.5.5v2.5a1 1 0 001 1h12a1 1 0 001-1v-2.5a.5.5 0 011 0v2.5a2 2 0 01-2 2H2a2 2 0 01-2-2v-2.5a.5.5 0 01.5-.5z"/>
              <path d="M7.646 1.146a.5.5 0 01.708 0l3 3a.5.5 0 01-.708.708L8.5 2.707V11.5a.5.5 0 01-1 0V2.707L5.354 4.854a.5.5 0 11-.708-.708l3-3z"/>
            </svg>
            Subir{{ localSelected.size > 0 ? ` (${localSelected.size})` : '' }}
          </button>
          <button
            class="ftp-xfer-btn download"
            @click="downloadSelected"
            :disabled="remoteSelected.size === 0 || transferring"
            :title="`Descargar ${remoteSelected.size} archivo(s) seleccionado(s)`"
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M.5 9.9a.5.5 0 01.5.5v2.5a1 1 0 001 1h12a1 1 0 001-1v-2.5a.5.5 0 011 0v2.5a2 2 0 01-2 2H2a2 2 0 01-2-2v-2.5a.5.5 0 01.5-.5z"/>
              <path d="M7.646 10.854a.5.5 0 00.708 0l3-3a.5.5 0 00-.708-.708L8.5 9.293V1.5a.5.5 0 00-1 0v7.793L5.354 7.146a.5.5 0 10-.708.708l3 3z"/>
            </svg>
            Descargar{{ remoteSelected.size > 0 ? ` (${remoteSelected.size})` : '' }}
          </button>
        </template>
        <span v-if="transferring && !uploadProgress" class="xfer-spin" />
      </div>

      <div class="ftp-status" v-if="statusMsg" :class="statusType">{{ statusMsg }}</div>
    </div>

    <!-- Dual pane -->
    <div class="ftp-main">

      <!-- ── LOCAL panel ── -->
      <div class="ftp-pane" :style="{ width: splitPos + '%' }">
        <div class="ftp-pane-header">
          <div class="pane-label">LOCAL</div>
          <div class="pane-nav">
            <button class="pane-nav-btn" @click="localBack" :disabled="localStack.length === 0" title="Atrás">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M11.354 1.646a.5.5 0 010 .708L5.707 8l5.647 5.646a.5.5 0 01-.708.708l-6-6a.5.5 0 010-.708l6-6a.5.5 0 01.708 0z"/></svg>
            </button>
            <button class="pane-nav-btn" @click="localUp" title="Carpeta superior">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M8 15a.5.5 0 00.5-.5V2.707l3.146 3.147a.5.5 0 00.708-.708l-4-4a.5.5 0 00-.708 0l-4 4a.5.5 0 10.708.708L7.5 2.707V14.5a.5.5 0 00.5.5z"/></svg>
            </button>
            <button class="pane-nav-btn" @click="localHome" title="Raíz del proyecto">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M8.354 1.146a.5.5 0 00-.708 0l-6 6A.5.5 0 002 8v7a.5.5 0 00.5.5h4a.5.5 0 00.5-.5v-4h2v4a.5.5 0 00.5.5h4a.5.5 0 00.5-.5V8a.5.5 0 00-.146-.354L13 6.5V2a.5.5 0 00-.5-.5h-1a.5.5 0 00-.5.5v1.293L8.354 1.146z"/></svg>
            </button>
            <button class="pane-nav-btn" @click="loadLocal(localPath)" title="Recargar">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M8 3a5 5 0 104.546 2.914.5.5 0 01.908-.417A6 6 0 118 2v1z"/><path d="M8 4.466V.534a.25.25 0 01.41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 018 4.466z"/></svg>
            </button>
          </div>
          <div class="pane-path" :title="localPath">{{ localPath.split('/').pop() || '/' }}</div>
          <div v-if="localSelected.size > 0" class="pane-sel-count">{{ localSelected.size }} sel.</div>
        </div>

        <div class="pane-hint" v-if="localItems.length > 0">
          <span>↑↓ navegar · Espacio seleccionar · Ctrl+A todo · U subir · Enter abrir</span>
        </div>

        <div
          ref="localBodyEl"
          class="pane-body" :class="{ 'pane-body--split': localPreview }"
          tabindex="0"
          @keydown="onLocalKeydown"
          @focus="localFocusIdx < 0 && localItems.length ? localFocusIdx = 0 : null"
        >
          <div v-if="localLoading" class="pane-loading"><span class="mini-spin" /> Cargando…</div>
          <template v-else>
            <div
              v-for="(e, i) in localItems" :key="e.path"
              class="pane-row"
              :class="{ selected: localSelected.has(e.path), is_dir: e.is_dir, focused: i === localFocusIdx }"
              @click="localFocusIdx = i; localClick(e, $event)"
              @dblclick="localDblclick(e)"
              :title="e.is_dir ? 'Enter: entrar · Espacio: seleccionar' : 'Espacio: seleccionar · Enter: abrir'"
            >
              <span class="row-check" :class="{ checked: localSelected.has(e.path) }">
                <svg v-if="localSelected.has(e.path)" width="9" height="9" viewBox="0 0 12 12" fill="currentColor"><path d="M10.28 2.28L4 8.56l-2.28-2.28a1 1 0 00-1.44 1.44l3 3a1 1 0 001.44 0l7-7a1 1 0 00-1.44-1.44z"/></svg>
              </span>
              <svg v-if="e.is_dir" class="row-icon" width="13" height="13" viewBox="0 0 16 16" fill="currentColor" style="color:#dcb67a">
                <path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.22.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5L6.34 1.5A1.75 1.75 0 004.99 1H1.75z"/>
              </svg>
              <svg v-else class="row-icon" width="11" height="13" viewBox="0 0 16 16" fill="currentColor" :style="{ color: fileColor(e.name) }">
                <path d="M14 4.5V14a2 2 0 01-2 2H4a2 2 0 01-2-2V2a2 2 0 012-2h5.5L14 4.5zm-3 0A1.5 1.5 0 019.5 3V1H4a1 1 0 00-1 1v12a1 1 0 001 1h8a1 1 0 001-1V4.5h-2z"/>
              </svg>
              <span class="row-name">{{ e.name }}</span>
              <button v-if="!e.is_dir" class="row-act-btn local-preview-btn"
                @click.stop="openLocalPreview(e)" title="Ver / Editar">
                <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M16 8s-3-5.5-8-5.5S0 8 0 8s3 5.5 8 5.5S16 8 16 8zM1.173 8a13.133 13.133 0 011.66-2.043C4.12 4.668 5.88 3.5 8 3.5c2.12 0 3.879 1.168 5.168 2.457A13.133 13.133 0 0114.828 8c-.058.087-.122.183-.195.288-.335.48-.83 1.12-1.465 1.755C11.879 11.332 10.119 12.5 8 12.5c-2.12 0-3.879-1.168-5.168-2.457A13.144 13.144 0 011.172 8z"/><circle cx="8" cy="8" r="2"/></svg>
              </button>
            </div>
          </template>
        </div>

        <!-- Local file preview / editor -->
        <div v-if="localPreview && localPreviewEntry" class="file-preview">
          <div class="preview-header">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" :style="{ color: fileColor(localPreviewEntry.name) }">
              <path d="M14 4.5V14a2 2 0 01-2 2H4a2 2 0 01-2-2V2a2 2 0 012-2h5.5L14 4.5zm-3 0A1.5 1.5 0 019.5 3V1H4a1 1 0 00-1 1v12a1 1 0 001 1h8a1 1 0 001-1V4.5h-2z"/>
            </svg>
            <span class="preview-filename">{{ localPreviewEntry.name }}</span>
            <div style="flex:1"/>
            <button class="preview-save-btn" @click="saveLocal">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M2 1a1 1 0 00-1 1v12a1 1 0 001 1h12a1 1 0 001-1V4.5a.5.5 0 00-.146-.354l-3-3A.5.5 0 0011.5 1H2zm5 1h1v3.5a.5.5 0 01-.5.5h-4a.5.5 0 01-.5-.5V2h4zm-2 8a2 2 0 114 0 2 2 0 01-4 0z"/></svg>
              Guardar
            </button>
            <button class="preview-close-btn" @click="localPreview = false; localContent = null">×</button>
          </div>
          <div v-if="localContent === null" class="preview-loading"><span class="mini-spin" /> Cargando…</div>
          <textarea v-else v-model="localContent" class="preview-textarea" spellcheck="false" />
        </div>
      </div>

      <!-- Splitter -->
      <div class="ftp-splitter" @mousedown="onSplitMousedown"><div class="splitter-handle" /></div>

      <!-- ── REMOTE panel ── -->
      <div class="ftp-pane ftp-pane-remote" :style="{ width: (100 - splitPos) + '%' }">
        <div class="ftp-pane-header">
          <div class="pane-label remote-label">SERVIDOR</div>
          <div class="pane-nav">
            <button class="pane-nav-btn" @click="remoteBack" :disabled="remoteStack.length === 0" title="Atrás">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M11.354 1.646a.5.5 0 010 .708L5.707 8l5.647 5.646a.5.5 0 01-.708.708l-6-6a.5.5 0 010-.708l6-6a.5.5 0 01.708 0z"/></svg>
            </button>
            <button class="pane-nav-btn" @click="loadRemote('/')" title="Raíz del servidor">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M8.354 1.146a.5.5 0 00-.708 0l-6 6A.5.5 0 002 8v7a.5.5 0 00.5.5h4a.5.5 0 00.5-.5v-4h2v4a.5.5 0 00.5.5h4a.5.5 0 00.5-.5V8a.5.5 0 00-.146-.354L13 6.5V2a.5.5 0 00-.5-.5h-1a.5.5 0 00-.5.5v1.293L8.354 1.146z"/></svg>
            </button>
            <button class="pane-nav-btn" @click="loadRemote(remotePath)" title="Recargar">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M8 3a5 5 0 104.546 2.914.5.5 0 01.908-.417A6 6 0 118 2v1z"/><path d="M8 4.466V.534a.25.25 0 01.41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 018 4.466z"/></svg>
            </button>
            <button class="pane-nav-btn" @click="showNewDir = 'remote'; newDirName = ''" title="Nueva carpeta remota">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
                <path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.22.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5L6.34 1.5A1.75 1.75 0 004.99 1H1.75z"/>
                <path d="M8 6.5a.5.5 0 011 0V8h1.5a.5.5 0 010 1H9v1.5a.5.5 0 01-1 0V9H6.5a.5.5 0 010-1H8V6.5z" fill="#0f1012"/>
              </svg>
            </button>
          </div>
          <div class="remote-hint">↑↓ · Espacio sel · Ctrl+A todo · D descargar · Supr eliminar · F2 renombrar</div>
          <div v-if="remoteSelected.size > 0" class="pane-sel-count">{{ remoteSelected.size }} sel.</div>
          <div class="pane-breadcrumb">
            <button v-for="(c, i) in remoteBreadcrumbs" :key="i"
              class="crumb-btn" :class="{ active: i === remoteBreadcrumbs.length - 1 }"
              @click="loadRemote(c.path)">{{ c.label }}</button>
          </div>
        </div>

        <div v-if="showNewDir === 'remote'" class="new-dir-bar">
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="color:#dcb67a"><path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.22.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5L6.34 1.5A1.75 1.75 0 004.99 1H1.75z"/></svg>
          <input v-model="newDirName" class="new-dir-input" placeholder="nombre-carpeta"
            @keydown.enter="createRemoteDir" @keydown.escape="showNewDir = null" autofocus />
          <button class="new-dir-ok" @click="createRemoteDir">OK</button>
          <button class="new-dir-cancel" @click="showNewDir = null">×</button>
        </div>

        <div
          ref="remoteBodyEl"
          class="pane-body" :class="{ 'pane-body--split': remotePreview }"
          tabindex="0"
          @keydown="onRemoteKeydown"
          @focus="remoteFocusIdx < 0 && remoteItems.length ? remoteFocusIdx = 0 : null"
        >
          <div v-if="remoteLoading" class="pane-loading"><span class="mini-spin" /> Cargando…</div>
          <div v-else-if="remoteError" class="pane-error">{{ remoteError }}</div>
          <template v-else>
            <div
              v-for="(e, i) in remoteItems" :key="e.path"
              class="pane-row"
              :class="{ selected: remoteSelected.has(e.path), is_dir: e.is_dir, focused: i === remoteFocusIdx }"
              @click="remoteFocusIdx = i; remoteClick(e, $event)"
              @dblclick="!e.is_dir && previewRemote(e)"
            >
              <template v-if="renaming?.entry.path === e.path">
                <input v-model="renaming.name" class="rename-input"
                  @keydown.enter="confirmRename" @keydown.escape="renaming = null" @click.stop autofocus />
                <button class="rename-ok" @click.stop="confirmRename">✓</button>
                <button class="rename-cancel" @click.stop="renaming = null">×</button>
              </template>
              <template v-else>
                <span class="row-check" :class="{ checked: remoteSelected.has(e.path) }">
                  <svg v-if="remoteSelected.has(e.path)" width="9" height="9" viewBox="0 0 12 12" fill="currentColor"><path d="M10.28 2.28L4 8.56l-2.28-2.28a1 1 0 00-1.44 1.44l3 3a1 1 0 001.44 0l7-7a1 1 0 00-1.44-1.44z"/></svg>
                </span>
                <svg v-if="e.is_dir" class="row-icon" width="13" height="13" viewBox="0 0 16 16" fill="currentColor" style="color:#dcb67a">
                  <path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.22.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5L6.34 1.5A1.75 1.75 0 004.99 1H1.75z"/>
                </svg>
                <svg v-else class="row-icon" width="11" height="13" viewBox="0 0 16 16" fill="currentColor" :style="{ color: fileColor(e.name) }">
                  <path d="M14 4.5V14a2 2 0 01-2 2H4a2 2 0 01-2-2V2a2 2 0 012-2h5.5L14 4.5zm-3 0A1.5 1.5 0 019.5 3V1H4a1 1 0 00-1 1v12a1 1 0 001 1h8a1 1 0 001-1V4.5h-2z"/>
                </svg>
                <span class="row-name">{{ e.name }}</span>
                <span class="row-size">{{ formatSize(e.size) }}</span>
                <div class="row-actions">
                  <button v-if="!e.is_dir" class="row-act-btn" @click.stop="previewRemote(e)" title="Ver / Editar">
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M16 8s-3-5.5-8-5.5S0 8 0 8s3 5.5 8 5.5S16 8 16 8zM1.173 8a13.133 13.133 0 011.66-2.043C4.12 4.668 5.88 3.5 8 3.5c2.12 0 3.879 1.168 5.168 2.457A13.133 13.133 0 0114.828 8c-.058.087-.122.183-.195.288-.335.48-.83 1.12-1.465 1.755C11.879 11.332 10.119 12.5 8 12.5c-2.12 0-3.879-1.168-5.168-2.457A13.144 13.144 0 011.172 8z"/><circle cx="8" cy="8" r="2"/></svg>
                  </button>
                  <button class="row-act-btn" @click.stop="startRename(e)" title="Renombrar">
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M12.854.146a.5.5 0 00-.707 0L10.5 1.793 14.207 5.5l1.647-1.646a.5.5 0 000-.708l-3-3zm.646 6.061L9.793 2.5 3.293 9H3.5a.5.5 0 01.5.5v.5h.5a.5.5 0 01.5.5v.5h.5a.5.5 0 01.5.5v.5h.5a.5.5 0 01.5.5v.207l6.5-6.5zm-7.468 7.468A.5.5 0 015.5 14H5v-.5a.5.5 0 01-.5-.5H4v-.5a.5.5 0 01-.5-.5H3v-.5a.5.5 0 01-.5-.5H2.5v-.5a.5.5 0 01.646-.473l.853.214 1.832-1.832 3.709 3.709-1.832 1.832.214.853z"/></svg>
                  </button>
                  <button class="row-act-btn del" @click.stop="remoteDelete(e)" title="Eliminar">
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M5.5 5.5A.5.5 0 016 6v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm2.5 0a.5.5 0 01.5.5v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm3 .5a.5.5 0 00-1 0v6a.5.5 0 001 0V6z"/><path fill-rule="evenodd" d="M14.5 3a1 1 0 01-1 1H13v9a2 2 0 01-2 2H5a2 2 0 01-2-2V4h-.5a1 1 0 010-2h4a1 1 0 011-1h2a1 1 0 011 1h4a1 1 0 011 1zM4.118 4L4 4.059V13a1 1 0 001 1h6a1 1 0 001-1V4.059L11.882 4H4.118zM2.5 3a.5.5 0 000 1h11a.5.5 0 000-1h-11z"/></svg>
                  </button>
                </div>
              </template>
            </div>
          </template>
        </div>

        <!-- Remote file preview / editor -->
        <div v-if="remotePreview && remotePreviewEntry" class="file-preview">
          <div class="preview-header">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" :style="{ color: fileColor(remotePreviewEntry.name) }">
              <path d="M14 4.5V14a2 2 0 01-2 2H4a2 2 0 01-2-2V2a2 2 0 012-2h5.5L14 4.5zm-3 0A1.5 1.5 0 019.5 3V1H4a1 1 0 00-1 1v12a1 1 0 001 1h8a1 1 0 001-1V4.5h-2z"/>
            </svg>
            <span class="preview-filename">{{ remotePreviewEntry.name }}</span>
            <span class="preview-size">{{ formatSize(remotePreviewEntry.size) }}</span>
            <div style="flex:1"/>
            <button class="preview-save-btn" @click="saveRemote">
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M2 1a1 1 0 00-1 1v12a1 1 0 001 1h12a1 1 0 001-1V4.5a.5.5 0 00-.146-.354l-3-3A.5.5 0 0011.5 1H2zm5 1h1v3.5a.5.5 0 01-.5.5h-4a.5.5 0 01-.5-.5V2h4zm-2 8a2 2 0 114 0 2 2 0 01-4 0z"/></svg>
              Guardar en servidor
            </button>
            <button class="preview-close-btn" @click="remotePreview = false; remoteContent = null">×</button>
          </div>
          <div v-if="remoteContent === null" class="preview-loading"><span class="mini-spin" /> Cargando…</div>
          <textarea v-else v-model="remoteContent" class="preview-textarea" spellcheck="false" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ftp-view { display:flex; flex-direction:column; height:100%; background:var(--bg-darkest); font-family:var(--font-ui); overflow:hidden; position:relative; }
.disconnected-overlay { position:absolute; inset:0; z-index:50; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:14px; background:rgba(10,12,14,0.88); backdrop-filter:blur(4px); color:var(--fg-muted); }
.disc-title { font-size:18px; font-weight:700; color:var(--fg); }
.disc-sub { font-size:12.5px; color:var(--fg-dim); text-align:center; line-height:1.7; }
.ftp-view.dragging { user-select:none; cursor:col-resize; }

/* Top bar */
.ftp-topbar { display:flex; align-items:center; gap:12px; padding:0 14px; height:40px; flex-shrink:0; background:var(--bg-dark); border-bottom:1px solid var(--border); }
.ftp-conn-badge { display:flex; align-items:center; gap:7px; font-size:13px; font-weight:600; color:var(--fg); }
.ftp-badge-proto { font-size:10px; color:var(--accent); background:rgba(46,158,135,0.12); border:1px solid rgba(46,158,135,0.25); border-radius:10px; padding:1px 7px; font-weight:700; }
.ftp-badge-name { color:var(--fg-bright); }

.ftp-transfer-actions { display:flex; align-items:center; gap:6px; }
.ftp-xfer-btn { display:flex; align-items:center; gap:5px; border:none; border-radius:6px; font-size:11.5px; font-weight:600; padding:5px 12px; cursor:pointer; transition:opacity 0.12s; }
.ftp-xfer-btn.upload { background:rgba(46,158,135,0.2); color:var(--accent); border:1px solid rgba(46,158,135,0.35); }
.ftp-xfer-btn.download { background:rgba(80,130,200,0.15); color:#79b8ff; border:1px solid rgba(80,130,200,0.3); }
.ftp-xfer-btn:disabled { opacity:0.35; cursor:default; }
.ftp-xfer-btn:not(:disabled):hover { opacity:0.8; }

.upload-progress { width:120px; height:6px; background:var(--bg-active); border-radius:3px; overflow:hidden; }
.upload-progress-bar { height:100%; background:var(--accent); border-radius:3px; transition:width 0.2s; }
.upload-count { font-size:11px; color:var(--accent); font-family:var(--font-mono); }

.xfer-spin { width:14px; height:14px; border:2px solid rgba(46,158,135,0.3); border-top-color:var(--accent); border-radius:50%; animation:spin 0.7s linear infinite; }
@keyframes spin { to { transform:rotate(360deg); } }

.ftp-status { font-size:11.5px; padding:4px 10px; border-radius:5px; animation:fadein 0.2s; }
.ftp-status.ok { background:rgba(46,158,135,0.12); color:var(--accent); border:1px solid rgba(46,158,135,0.25); }
.ftp-status.err { background:rgba(248,81,73,0.12); color:#f85149; border:1px solid rgba(248,81,73,0.25); }
@keyframes fadein { from { opacity:0; transform:translateY(-2px); } to { opacity:1; } }

/* Layout */
.ftp-main { flex:1; display:flex; overflow:hidden; min-height:0; }
.ftp-pane { display:flex; flex-direction:column; overflow:hidden; flex-shrink:0; }

/* Pane header */
.ftp-pane-header { display:flex; align-items:center; gap:5px; padding:0 10px; height:36px; flex-shrink:0; background:var(--bg-panel); border-bottom:1px solid var(--border); overflow:hidden; }
.pane-label { font-size:9px; font-weight:800; letter-spacing:1.2px; color:var(--fg-muted); text-transform:uppercase; flex-shrink:0; background:var(--bg-mid); border-radius:3px; padding:2px 6px; }
.remote-label { color:var(--accent); }
.pane-nav { display:flex; gap:1px; flex-shrink:0; }
.pane-nav-btn { width:22px; height:22px; background:none; border:none; color:var(--fg-muted); border-radius:4px; cursor:pointer; display:flex; align-items:center; justify-content:center; transition:all 0.1s; }
.pane-nav-btn:hover:not(:disabled) { background:var(--bg-hover); color:var(--fg); }
.pane-nav-btn:disabled { opacity:0.3; cursor:default; }
.pane-path { font-size:11px; color:var(--fg-dim); flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.pane-sel-count { font-size:10px; background:rgba(46,158,135,0.2); color:var(--accent); border-radius:10px; padding:1px 7px; flex-shrink:0; font-weight:700; }

.remote-hint { font-size:9.5px; color:var(--fg-muted); white-space:nowrap; opacity:0.6; flex-shrink:0; }
.pane-breadcrumb { display:flex; align-items:center; flex:1; overflow:hidden; }
.crumb-btn { background:none; border:none; color:var(--fg-muted); font-size:11px; cursor:pointer; padding:2px 3px; border-radius:3px; white-space:nowrap; transition:color 0.1s; }
.crumb-btn:not(.active)::after { content:'/'; color:var(--border); margin-left:1px; }
.crumb-btn:hover { color:var(--fg); }
.crumb-btn.active { color:var(--fg-bright); font-weight:600; cursor:default; }

/* Hint bar */
.pane-hint { padding:3px 12px; font-size:10px; color:var(--fg-muted); background:var(--bg-dark); border-bottom:1px solid var(--border); flex-shrink:0; opacity:0.7; }

/* New dir */
.new-dir-bar { display:flex; align-items:center; gap:6px; padding:6px 10px; background:var(--bg-dark); border-bottom:1px solid var(--border); flex-shrink:0; }
.new-dir-input { flex:1; background:var(--bg-mid); border:1px solid var(--accent); border-radius:4px; color:var(--fg); font-size:12px; padding:3px 7px; outline:none; }
.new-dir-ok,.new-dir-cancel { background:none; border:none; cursor:pointer; font-size:13px; padding:2px 6px; border-radius:4px; }
.new-dir-ok { color:var(--accent); }
.new-dir-cancel { color:var(--fg-muted); }

/* Pane body */
.pane-body { flex:1; overflow-y:auto; overflow-x:hidden; outline:none; }
.pane-body--split { flex:none; height:55%; }
.pane-body:focus { box-shadow:inset 0 0 0 1px rgba(46,158,135,0.35); }
.pane-body::-webkit-scrollbar { width:5px; }
.pane-body::-webkit-scrollbar-thumb { background:var(--bg-active); border-radius:3px; }

.pane-loading,.pane-error { display:flex; align-items:center; gap:8px; padding:16px; font-size:12px; color:var(--fg-muted); }
.pane-error { color:#f85149; }
.mini-spin { width:12px; height:12px; border:2px solid rgba(46,158,135,0.3); border-top-color:var(--accent); border-radius:50%; animation:spin 0.7s linear infinite; flex-shrink:0; }

/* Rows */
.pane-row { display:flex; align-items:center; gap:6px; padding:3px 8px; font-size:12px; color:var(--fg-dim); cursor:pointer; transition:background 0.08s; user-select:none; border-radius:3px; margin:1px 4px; }
.pane-row:hover { background:var(--bg-hover); color:var(--fg); }
.pane-row:hover .row-act-btn,.pane-row:hover .local-preview-btn { opacity:1; }
.pane-row.selected { background:rgba(46,158,135,0.14); color:var(--fg-bright); }
.pane-row.focused { outline:1px solid rgba(46,158,135,0.5); outline-offset:-1px; }
.pane-row.focused:not(.selected) { background:var(--bg-hover); }
.pane-row.is_dir .row-name { font-weight:600; color:var(--fg); }

.row-check { width:14px; height:14px; border:1px solid var(--border); border-radius:3px; flex-shrink:0; display:flex; align-items:center; justify-content:center; transition:all 0.1s; }
.row-check.checked { background:var(--accent); border-color:var(--accent); color:#fff; }
.row-icon { flex-shrink:0; }
.row-name { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.row-size { font-size:10px; color:var(--fg-muted); flex-shrink:0; min-width:52px; text-align:right; }

.row-actions { display:flex; gap:1px; opacity:0; transition:opacity 0.1s; flex-shrink:0; }
.row-act-btn,.local-preview-btn { width:20px; height:20px; background:none; border:none; border-radius:3px; color:var(--fg-muted); cursor:pointer; display:flex; align-items:center; justify-content:center; transition:all 0.1s; opacity:0; }
.row-act-btn:hover { background:var(--bg-active); color:var(--fg); }
.row-act-btn.del:hover { color:#f85149; }
.local-preview-btn:hover { color:var(--accent) !important; }

/* Rename */
.rename-input { flex:1; background:var(--bg-mid); border:1px solid var(--accent); border-radius:4px; color:var(--fg); font-size:12px; padding:2px 6px; outline:none; }
.rename-ok,.rename-cancel { background:none; border:none; cursor:pointer; font-size:13px; padding:2px 5px; border-radius:3px; }
.rename-ok { color:var(--accent); }
.rename-cancel { color:var(--fg-muted); }

/* Splitter */
.ftp-splitter { width:5px; flex-shrink:0; cursor:col-resize; background:var(--border); display:flex; align-items:center; justify-content:center; transition:background 0.12s; }
.ftp-splitter:hover { background:var(--accent); }
.splitter-handle { width:3px; height:40px; border-radius:2px; background:rgba(255,255,255,0.08); }

/* File preview (shared local + remote) */
.file-preview { border-top:1px solid var(--border); flex-shrink:0; display:flex; flex-direction:column; height:45%; background:var(--bg-darkest); }
.preview-header { display:flex; align-items:center; gap:7px; padding:5px 10px; background:var(--bg-dark); border-bottom:1px solid var(--border); flex-shrink:0; }
.preview-filename { font-size:12px; font-weight:600; color:var(--fg); }
.preview-size { font-size:10.5px; color:var(--fg-muted); }
.preview-save-btn { display:flex; align-items:center; gap:5px; background:rgba(46,158,135,0.15); border:1px solid rgba(46,158,135,0.3); border-radius:5px; color:var(--accent); font-size:11px; font-weight:600; padding:3px 10px; cursor:pointer; transition:opacity 0.12s; }
.preview-save-btn:hover { opacity:0.8; }
.preview-close-btn { background:none; border:none; color:var(--fg-muted); font-size:17px; cursor:pointer; line-height:1; width:22px; height:22px; border-radius:4px; display:flex; align-items:center; justify-content:center; }
.preview-close-btn:hover { background:var(--bg-hover); color:var(--fg); }
.preview-loading { display:flex; align-items:center; gap:8px; padding:16px; font-size:12px; color:var(--fg-muted); }
.preview-textarea { flex:1; background:transparent; border:none; color:var(--fg); font-family:var(--font-mono); font-size:12.5px; line-height:1.6; padding:10px 14px; resize:none; outline:none; overflow:auto; }
.preview-textarea::-webkit-scrollbar { width:6px; }
.preview-textarea::-webkit-scrollbar-thumb { background:var(--bg-active); border-radius:3px; }
</style>
