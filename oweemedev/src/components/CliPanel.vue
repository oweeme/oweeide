<script setup lang="ts">
/**
 * CliPanel — panel embebido con xterm PTY para Claude Code CLI y Gemini CLI.
 * Se abre como sidebar view (igual que FileExplorer) y corre el CLI directamente.
 */
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { Terminal as XTerm } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useEditorStore } from '../composables/useEditorStore'
import '@xterm/xterm/css/xterm.css'

const props = defineProps<{
  cli: 'claude' | 'gemini'
}>()

const emit = defineEmits<{ 'open-file': [path: string] }>()

const store = useEditorStore()
const termEl = ref<HTMLElement | null>(null)
const isRunning = ref(false)
const sessionId = `clipanel-${props.cli}-${Date.now()}`

let xterm: XTerm | null = null
let fitAddon: FitAddon | null = null
let unlistenData: UnlistenFn | null = null
let unlistenExit: UnlistenFn | null = null

const CLI_META = {
  claude: {
    label: 'Claude Code',
    icon: '◆',
    color: '#d97706',
    bgColor: 'rgba(217,119,6,0.08)',
    borderColor: 'rgba(217,119,6,0.25)',
    command: 'claude',
    tip: 'Usa tu suscripción Claude Pro sin API key adicional',
    shortcuts: ['↑↓ historial', 'Tab completar', '/help comandos'],
  },
  gemini: {
    label: 'Gemini CLI',
    icon: '◈',
    color: '#4285f4',
    bgColor: 'rgba(66,133,244,0.08)',
    borderColor: 'rgba(66,133,244,0.25)',
    command: 'gemini',
    tip: 'Google Gemini CLI — usa tu cuenta de Google',
    shortcuts: ['↑↓ historial', 'Tab completar', '/help comandos'],
  },
}

const meta = CLI_META[props.cli]

// OSC parser (same as Terminal.vue — for open-in-editor support)
let oscBuf = ''
let inOsc = false
function processOutput(data: string) {
  let visible = ''
  for (let i = 0; i < data.length; i++) {
    const ch = data[i]
    if (inOsc) {
      if (ch === '\x07') {
        if (oscBuf.startsWith('6634;')) emit('open-file', oscBuf.slice(5))
        oscBuf = ''; inOsc = false
      } else if (ch === '\x1b' && data[i + 1] === '\\') {
        if (oscBuf.startsWith('6634;')) emit('open-file', oscBuf.slice(5))
        oscBuf = ''; inOsc = false; i++
      } else { oscBuf += ch }
    } else if (ch === '\x1b' && data[i + 1] === ']') {
      inOsc = true; oscBuf = ''; i++
    } else { visible += ch }
  }
  if (visible) xterm?.write(visible)
}

onMounted(async () => {
  xterm = new XTerm({
    theme: {
      background:          '#0d1117',
      foreground:          '#e6edf3',
      cursor:              props.cli === 'claude' ? '#d97706' : '#4285f4',
      cursorAccent:        '#0d1117',
      selectionBackground: props.cli === 'claude' ? 'rgba(217,119,6,0.25)' : 'rgba(66,133,244,0.25)',
      black:   '#21262d', red:     '#f85149',
      green:   '#3fb950', yellow:  '#d29922',
      blue:    '#79c0ff', magenta: '#d2a8ff',
      cyan:    '#56d8e4', white:   '#c9d1d9',
      brightBlack:   '#484f58', brightRed:     '#ff7b72',
      brightGreen:   '#56d364', brightYellow:  '#e3b341',
      brightBlue:    '#79c0ff', brightMagenta: '#d2a8ff',
      brightCyan:    '#56d8e4', brightWhite:   '#f0f6fc',
    },
    fontFamily: '"JetBrains Mono", "Fira Code", Consolas, monospace',
    fontSize: 12.5,
    lineHeight: 1.5,
    cursorBlink: true,
    cursorStyle: 'bar',
    scrollback: 10000,
  })

  fitAddon = new FitAddon()
  xterm.loadAddon(fitAddon)
  xterm.loadAddon(new WebLinksAddon())

  if (termEl.value) {
    xterm.open(termEl.value)
    setTimeout(() => { fitAddon?.fit() }, 80)
  }

  xterm.onData((data) => {
    invoke('pty_write', { id: sessionId, data }).catch(() => {})
  })

  unlistenData = await listen<string>(`pty:${sessionId}`, (ev) => {
    processOutput(ev.payload)
  })

  unlistenExit = await listen<string>(`pty-exit:${sessionId}`, () => {
    isRunning.value = false
    xterm?.writeln(`\r\n\x1b[33m[${meta.label} cerrado — presiona Enter para reiniciar]\x1b[0m`)
    xterm?.onData(() => { if (!isRunning.value) startCli() })
  })

  const ro = new ResizeObserver(() => {
    setTimeout(async () => {
      fitAddon?.fit()
      const raw = fitAddon?.proposeDimensions()
      const cols = Math.max(10, Math.round(raw?.cols ?? 0) || 80)
      const rows = Math.max(5,  Math.round(raw?.rows ?? 0) || 24)
      await invoke('pty_resize', { id: sessionId, cols, rows }).catch(() => {})
    }, 30)
  })
  if (termEl.value) ro.observe(termEl.value)
  onBeforeUnmount(() => ro.disconnect())

  await startCli()
})

onBeforeUnmount(async () => {
  unlistenData?.()
  unlistenExit?.()
  await invoke('pty_kill', { id: sessionId }).catch(() => {})
  xterm?.dispose()
})

async function startCli() {
  if (isRunning.value) return
  const el = termEl.value
  const isVisible = el != null && el.offsetWidth > 0 && el.offsetHeight > 0
  let cols = 80, rows = 24
  if (isVisible) {
    fitAddon?.fit()
    await new Promise(r => setTimeout(r, 40))
    const raw = fitAddon?.proposeDimensions()
    cols = Math.max(10, Math.round(raw?.cols ?? 80) || 80)
    rows = Math.max(5,  Math.round(raw?.rows ?? 24) || 24)
  }

  const cwd = store.state.rootPath || (await invoke<string>('get_home_dir').catch(() => '/'))

  try {
    await invoke('pty_create', {
      id: sessionId,
      cwd,
      cols,
      rows,
      command: meta.command,
    })
    isRunning.value = true
  } catch (e: any) {
    xterm?.writeln(`\r\n\x1b[31mError al iniciar ${meta.label}: ${e}\x1b[0m`)
    xterm?.writeln(`\r\n\x1b[33mAsegúrate de tener instalado '${meta.command}':\x1b[0m`)
    if (props.cli === 'claude') {
      xterm?.writeln('\x1b[36m  npm install -g @anthropic-ai/claude-code\x1b[0m')
    } else {
      xterm?.writeln('\x1b[36m  npm install -g @google/generative-ai-cli\x1b[0m')
    }
  }
}

async function restartCli() {
  await invoke('pty_kill', { id: sessionId }).catch(() => {})
  isRunning.value = false
  xterm?.clear()
  await startCli()
}

// Send active file context as a message to the CLI
async function sendFileContext() {
  const tab = store.activeTab()
  if (!tab || tab.type !== 'code') return
  const sel = store.state.selectedText.trim()
  const text = sel
    ? `Mira este código seleccionado de ${tab.name}:\n\`\`\`${tab.language}\n${sel}\n\`\`\``
    : `Mira este archivo ${tab.name}:\n\`\`\`${tab.language}\n${tab.content.slice(0, 2000)}\n\`\`\``
  // Type it into the terminal (the CLI reads stdin)
  invoke('pty_write', { id: sessionId, data: text }).catch(() => {})
  xterm?.focus()
}
</script>

<template>
  <div class="cli-panel">
    <!-- Header -->
    <div class="cli-header" :style="{ borderBottomColor: meta.borderColor }">
      <div class="cli-header-left">
        <span class="cli-icon" :style="{ color: meta.color }">{{ meta.icon }}</span>
        <span class="cli-title">{{ meta.label }}</span>
        <span class="cli-status" :class="isRunning ? 'cli-status--on' : 'cli-status--off'">
          {{ isRunning ? 'activo' : 'inactivo' }}
        </span>
      </div>
      <div class="cli-header-right">
        <!-- Send file context button -->
        <button
          class="cli-hdr-btn"
          @click="sendFileContext"
          :disabled="!store.activeTab() || store.activeTab()?.type !== 'code'"
          :title="store.state.selectedText ? 'Enviar selección al CLI' : 'Enviar archivo activo al CLI'"
        >
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 0h5.293A1 1 0 0 1 10 .293L13.707 4a1 1 0 0 1 .293.707V14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2zm5.5 1.5v2a1 1 0 0 0 1 1h2L9.5 1.5z"/>
          </svg>
          {{ store.state.selectedText ? 'Enviar selección' : 'Enviar archivo' }}
        </button>
        <!-- Restart button -->
        <button class="cli-hdr-btn cli-hdr-btn--icon" @click="restartCli" title="Reiniciar">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
            <path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2v1z"/>
            <path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466z"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Tip bar -->
    <div class="cli-tip" :style="{ background: meta.bgColor, borderBottomColor: meta.borderColor }">
      <span class="cli-tip-icon" :style="{ color: meta.color }">💡</span>
      <span>{{ meta.tip }}</span>
      <span v-for="s in meta.shortcuts" :key="s" class="cli-shortcut">{{ s }}</span>
    </div>

    <!-- Context indicator -->
    <div v-if="store.activeTab()?.type === 'code'" class="cli-ctx-bar">
      <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" style="flex-shrink:0;opacity:.6">
        <path d="M4 0h5.293A1 1 0 0 1 10 .293L13.707 4a1 1 0 0 1 .293.707V14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2zm5.5 1.5v2a1 1 0 0 0 1 1h2L9.5 1.5z"/>
      </svg>
      <span>
        {{ store.state.selectedText ? `Selección en ${store.activeTab()?.name}` : store.activeTab()?.name }}
        — clic en "Enviar" para pasar contexto al CLI
      </span>
    </div>

    <!-- xterm terminal -->
    <div ref="termEl" class="cli-term" />
  </div>
</template>

<style scoped>
.cli-panel {
  display: flex; flex-direction: column; height: 100%;
  background: #0d1117; font-family: var(--font-ui); overflow: hidden;
}

.cli-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 7px 10px; border-bottom: 1px solid;
  flex-shrink: 0; background: #161b22;
}
.cli-header-left { display: flex; align-items: center; gap: 7px; }
.cli-icon { font-size: 15px; }
.cli-title { font-size: 13px; font-weight: 700; color: #e6edf3; }
.cli-status {
  font-size: 9.5px; font-weight: 600; border-radius: 8px;
  padding: 1px 7px; border: 1px solid;
}
.cli-status--on  { color: #3fb950; border-color: rgba(63,185,80,.4); background: rgba(63,185,80,.1); }
.cli-status--off { color: #6e7681; border-color: rgba(110,118,129,.4); background: rgba(110,118,129,.1); }

.cli-header-right { display: flex; gap: 4px; align-items: center; }
.cli-hdr-btn {
  display: flex; align-items: center; gap: 5px;
  background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.1);
  border-radius: 5px; color: #8b949e; font-size: 11px;
  padding: 4px 8px; cursor: pointer; transition: all 0.12s; white-space: nowrap;
}
.cli-hdr-btn:hover:not(:disabled) { background: rgba(255,255,255,0.12); color: #e6edf3; }
.cli-hdr-btn:disabled { opacity: 0.4; cursor: default; }
.cli-hdr-btn--icon { padding: 4px 6px; }

.cli-tip {
  display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
  padding: 5px 10px; border-bottom: 1px solid;
  font-size: 10.5px; color: #8b949e; flex-shrink: 0;
}
.cli-tip-icon { font-size: 11px; }
.cli-shortcut {
  background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.1);
  border-radius: 4px; padding: 1px 6px; font-size: 10px; color: #6e7681;
  font-family: var(--font-mono);
}

.cli-ctx-bar {
  display: flex; align-items: center; gap: 5px;
  padding: 4px 10px; font-size: 10px; color: #6e7681;
  background: rgba(255,255,255,0.03); border-bottom: 1px solid rgba(255,255,255,0.06);
  flex-shrink: 0;
}

.cli-term {
  flex: 1; min-height: 0; padding: 6px 0 0 0;
  /* xterm.js needs explicit sizing */
  overflow: hidden;
}

/* xterm overrides */
.cli-term :deep(.xterm) { height: 100%; }
.cli-term :deep(.xterm-viewport) { overflow-y: auto !important; }
</style>
