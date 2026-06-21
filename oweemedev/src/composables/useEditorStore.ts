import { reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export type TabType = 'code' | 'image' | 'database' | 'ftp' | 'api' | 'ftp-file'

export interface Tab {
  path: string
  name: string
  content: string
  modified: boolean
  language: string
  type: TabType
  // database tab extras
  connId?: string
  tableName?: string
  driver?: string
  // ftp tab extras
  ftpConnId?: string
  ftpConnName?: string
  ftpProtocol?: string
  // ftp-file tab extras (remote file opened in editor)
  ftpFilePath?: string   // remote path on server
  // api tab extras
  apiRequestId?: string
}

interface EditorState {
  rootPath: string
  tabs: Tab[]
  activeTabPath: string | null
  cursorLine: number
  cursorCol: number
}

const IMAGE_EXTS = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'ico', 'bmp', 'avif'])

const state = reactive<EditorState>({
  rootPath: '',
  tabs: [],
  activeTabPath: null,
  cursorLine: 1,
  cursorCol: 1,
})

function detectLanguage(filename: string): string {
  // Exact filename matches for dotfiles and well-known config files
  const exactMap: Record<string, string> = {
    '.htaccess': 'apache', '.env': 'shell', '.envrc': 'shell',
    '.prettierrc': 'json', '.eslintrc': 'json', '.babelrc': 'json',
    '.stylelintrc': 'json', '.browserslistrc': 'text',
    'dockerfile': 'shell', 'makefile': 'shell', 'gemfile': 'ruby',
    '.gitignore': 'shell', '.gitattributes': 'text', '.editorconfig': 'toml',
    '.npmrc': 'toml', '.yarnrc': 'yaml',
  }
  const lower = filename.toLowerCase()
  if (exactMap[lower]) return exactMap[lower]

  const ext = filename.split('.').pop()?.toLowerCase() ?? ''
  const map: Record<string, string> = {
    ts: 'typescript', tsx: 'typescript',
    js: 'javascript', jsx: 'javascript', mjs: 'javascript', cjs: 'javascript',
    vue: 'vue', php: 'php', go: 'go',
    html: 'html', htm: 'html',
    css: 'css', scss: 'css', sass: 'css',
    json: 'json', jsonc: 'json', json5: 'json', md: 'markdown',
    rs: 'rust', py: 'python',
    sql: 'sql', sh: 'shell', bash: 'shell', zsh: 'shell',
    toml: 'toml', yaml: 'yaml', yml: 'yaml',
    xml: 'xml', svg: 'xml', xhtml: 'xml',
    cpp: 'cpp', cc: 'cpp', c: 'cpp', h: 'cpp', hpp: 'cpp',
    env: 'shell',
  }
  return map[ext] ?? 'text'
}

function isImage(filename: string): boolean {
  const ext = filename.split('.').pop()?.toLowerCase() ?? ''
  return IMAGE_EXTS.has(ext)
}

export function useEditorStore() {
  function setRootPath(p: string) {
    state.rootPath = p
  }

  async function openFile(path: string, _line?: number) {
    const existing = state.tabs.find(t => t.path === path)
    if (existing) { state.activeTabPath = path; return }

    const name = path.split('/').pop() ?? path

    if (isImage(name)) {
      state.tabs.push({ path, name, content: '', modified: false, language: 'image', type: 'image' })
      state.activeTabPath = path
      return
    }

    try {
      const content = await invoke<string>('open_file', { path })
      state.tabs.push({ path, name, content, modified: false, language: detectLanguage(name), type: 'code' })
      state.activeTabPath = path
    } catch (e) {
      console.error('Failed to open file:', e)
    }
  }

  async function saveFile(path: string) {
    const tab = state.tabs.find(t => t.path === path)
    if (!tab || tab.type === 'image') return
    if (tab.type === 'ftp-file') {
      await invoke('remote_write_file', { id: tab.ftpConnId!, path: tab.ftpFilePath!, content: tab.content })
      tab.modified = false
      return
    }
    if (tab.type === 'api' || tab.type === 'database' || tab.type === 'ftp') return
    await invoke('save_file', { path, content: tab.content })
    tab.modified = false
  }

  async function saveActiveFile() {
    if (state.activeTabPath) await saveFile(state.activeTabPath)
  }

  function updateContent(path: string, content: string) {
    const tab = state.tabs.find(t => t.path === path)
    if (tab && (tab.type === 'code' || tab.type === 'ftp-file' || tab.type === 'api')) { tab.content = content; tab.modified = true }
  }

  function closeTab(path: string) {
    const idx = state.tabs.findIndex(t => t.path === path)
    if (idx === -1) return
    state.tabs.splice(idx, 1)
    if (state.activeTabPath === path) {
      state.activeTabPath = state.tabs[Math.max(0, idx - 1)]?.path ?? null
    }
  }

  function setActive(path: string) { state.activeTabPath = path }
  function setCursor(line: number, col: number) { state.cursorLine = line; state.cursorCol = col }
  const activeTab = () => state.tabs.find(t => t.path === state.activeTabPath) ?? null

  function renameTab(oldPath: string, newPath: string, newName: string) {
    const tab = state.tabs.find(t => t.path === oldPath)
    if (!tab) return
    tab.path = newPath
    tab.name = newName
    if (state.activeTabPath === oldPath) state.activeTabPath = newPath
  }

  function openFtpConn(connId: string, connName: string, protocol: string) {
    const path = `ftp://${connId}`
    const existing = state.tabs.find(t => t.path === path)
    if (existing) { state.activeTabPath = path; return }
    state.tabs.push({ path, name: connName, content: '', modified: false, language: 'text', type: 'ftp', ftpConnId: connId, ftpConnName: connName, ftpProtocol: protocol })
    state.activeTabPath = path
  }

  function openDbTable(connId: string, tableName: string, _connName: string, driver: string) {
    const path = `db://${connId}/${tableName}`
    const existing = state.tabs.find(t => t.path === path)
    if (existing) { state.activeTabPath = path; return }
    state.tabs.push({ path, name: tableName, content: '', modified: false, language: 'sql', type: 'database', connId, tableName, driver })
    state.activeTabPath = path
  }

  function openFtpFile(connId: string, remotePath: string, fileName: string, content: string) {
    const path = `ftpfile://${connId}${remotePath}`
    const existing = state.tabs.find(t => t.path === path)
    if (existing) { state.activeTabPath = path; return }
    state.tabs.push({
      path, name: fileName, content, modified: false,
      language: detectLanguage(fileName), type: 'ftp-file',
      ftpConnId: connId, ftpFilePath: remotePath,
    })
    state.activeTabPath = path
  }

  function openApiRequest(requestId: string, name: string, initialJson?: string) {
    const path = `api://${requestId}`
    const existing = state.tabs.find(t => t.path === path)
    if (existing) { state.activeTabPath = path; return }
    state.tabs.push({ path, name, content: initialJson ?? '{}', modified: false, language: 'json', type: 'api', apiRequestId: requestId })
    state.activeTabPath = path
  }

  function closeTabByPath(path: string) {
    // Close any tab whose path starts with the deleted path (handles folder deletes)
    const toClose = state.tabs.filter(t => t.path === path || t.path.startsWith(path + '/'))
    toClose.forEach(t => closeTab(t.path))
  }

  return { state, activeTab, setRootPath, openFile, saveFile, saveActiveFile, updateContent, closeTab, closeTabByPath, renameTab, setActive, setCursor, openDbTable, openFtpConn, openFtpFile, openApiRequest }
}
