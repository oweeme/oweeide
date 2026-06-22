<script setup lang="ts">
defineEmits<{ close: [] }>()

const GROUPS = [
  {
    title: 'Editor',
    items: [
      { keys: ['Ctrl', 'S'],           desc: 'Save file' },
      { keys: ['Ctrl', 'F'],           desc: 'Open / close search panel' },
      { keys: ['Esc'],                  desc: 'Close search panel' },
      { keys: ['F3'],                   desc: 'Find next match' },
      { keys: ['Shift', 'F3'],          desc: 'Find previous match' },
      { keys: ['Ctrl', 'H'],            desc: 'Open search & replace' },
      { keys: ['Ctrl', 'Z'],            desc: 'Undo' },
      { keys: ['Ctrl', 'Y'],            desc: 'Redo' },
      { keys: ['Ctrl', '/'],            desc: 'Toggle comment' },
      { keys: ['Ctrl', 'D'],            desc: 'Select next occurrence' },
      { keys: ['Alt', 'Z'],             desc: 'Toggle word wrap' },
      { keys: ['Ctrl', '+'],            desc: 'Zoom in (font size)' },
      { keys: ['Ctrl', '-'],            desc: 'Zoom out (font size)' },
      { keys: ['Ctrl', '0'],            desc: 'Reset font size' },
      { keys: ['Tab'],                  desc: 'Indent selection' },
      { keys: ['Shift', 'Tab'],         desc: 'Unindent selection' },
    ],
  },
  {
    title: 'Tabs',
    items: [
      { keys: ['Ctrl', 'W'],            desc: 'Close active tab' },
      { keys: ['Ctrl', 'Tab'],          desc: 'Next tab' },
      { keys: ['Ctrl', 'Shift', 'Tab'], desc: 'Previous tab' },
    ],
  },
  {
    title: 'Interface',
    items: [
      { keys: ['Ctrl', 'B'],            desc: 'Mostrar / ocultar sidebar' },
      { keys: ['Ctrl', 'Shift', 'F'],   desc: 'Buscar en archivos (sidebar)' },
      { keys: ['Ctrl', '`'],            desc: 'Mostrar / ocultar terminal' },
      { keys: ['Ctrl', 'Shift', 'A'],   desc: 'Abrir AI Assistant (sidebar)' },
      { keys: ['Ctrl', 'Shift', 'C'],   desc: 'Abrir Claude Code CLI (workspace)' },
      { keys: ['Ctrl', 'Shift', 'G'],   desc: 'Abrir Gemini CLI (workspace)' },
    ],
  },
  {
    title: 'File Explorer',
    items: [
      { keys: ['F2'],                   desc: 'Rename file / folder' },
      { keys: ['Esc'],                  desc: 'Cancel rename / create' },
      { keys: ['Enter'],                desc: 'Confirm rename / create' },
    ],
  },
  {
    title: 'Terminal',
    items: [
      { keys: ['Ctrl', 'C'],            desc: 'Interrupt running process' },
      { keys: ['Ctrl', 'D'],            desc: 'Send EOF / close session' },
      { keys: ['↑ / ↓'],               desc: 'Navigate command history' },
    ],
  },
]
</script>

<template>
  <div class="sc-overlay" @click.self="$emit('close')">
    <div class="sc-modal">
      <div class="sc-header">
        <span class="sc-title">Keyboard Shortcuts</span>
        <button class="sc-close" @click="$emit('close')">×</button>
      </div>

      <div class="sc-body">
        <div v-for="g in GROUPS" :key="g.title" class="sc-group">
          <div class="sc-group-title">{{ g.title.toUpperCase() }}</div>
          <div class="sc-row" v-for="item in g.items" :key="item.desc">
            <span class="sc-desc">{{ item.desc }}</span>
            <span class="sc-keys">
              <kbd v-for="(k, i) in item.keys" :key="i" class="sc-key">{{ k }}</kbd>
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sc-overlay {
  position: fixed; inset: 0; z-index: 99999;
  background: rgba(0,0,0,0.65); backdrop-filter: blur(4px);
  display: flex; align-items: center; justify-content: center;
}
.sc-modal {
  background: var(--bg-panel); border: 1px solid var(--border);
  border-radius: 14px; width: 520px; max-height: 80vh;
  display: flex; flex-direction: column;
  box-shadow: 0 24px 80px rgba(0,0,0,0.8);
  font-family: var(--font-ui);
}
.sc-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 16px 20px 12px; border-bottom: 1px solid var(--border); flex-shrink: 0;
}
.sc-title { font-size: 14px; font-weight: 700; color: var(--fg-bright); }
.sc-close {
  background: none; border: none; color: var(--fg-muted);
  font-size: 20px; cursor: pointer; width: 28px; height: 28px;
  border-radius: 6px; display: flex; align-items: center; justify-content: center;
}
.sc-close:hover { background: var(--bg-hover); color: var(--fg); }

.sc-body { overflow-y: auto; padding: 12px 20px 16px; }
.sc-body::-webkit-scrollbar { width: 4px; }
.sc-body::-webkit-scrollbar-thumb { background: var(--bg-active); border-radius: 2px; }

.sc-group { margin-bottom: 18px; }
.sc-group-title {
  font-size: 10px; font-weight: 700; letter-spacing: 1.2px;
  color: var(--accent); margin-bottom: 8px;
}
.sc-row {
  display: flex; align-items: center; justify-content: space-between;
  padding: 5px 0; border-bottom: 1px solid rgba(255,255,255,0.04);
}
.sc-row:last-child { border-bottom: none; }
.sc-desc { font-size: 12.5px; color: var(--fg-dim); }
.sc-keys { display: flex; align-items: center; gap: 3px; }
.sc-key {
  display: inline-flex; align-items: center; justify-content: center;
  background: var(--bg-mid); border: 1px solid var(--border);
  border-bottom-width: 2px; border-radius: 4px;
  padding: 2px 7px; font-size: 11px; font-family: var(--font-mono);
  color: var(--fg); min-width: 22px;
}
</style>
