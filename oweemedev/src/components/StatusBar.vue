<script setup lang="ts">
import { computed } from 'vue'
import { useEditorStore } from '../composables/useEditorStore'

const store = useEditorStore()

const activeTab = computed(() => store.activeTab())
const langLabel = computed(() => {
  if (!activeTab.value) return 'Plain Text'
  const lang = activeTab.value.language
  const labels: Record<string, string> = {
    typescript: 'TypeScript', javascript: 'JavaScript',
    vue: 'Vue', php: 'PHP', go: 'Go', html: 'HTML',
    css: 'CSS', json: 'JSON', markdown: 'Markdown',
    rust: 'Rust', python: 'Python', sql: 'SQL', cpp: 'C++',
    text: 'Plain Text',
  }
  return labels[lang] ?? lang
})
</script>

<template>
  <div class="statusbar">
    <div class="statusbar-left">
      <span class="statusbar-item statusbar-item--accent">
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
        </svg>
        OweemeIDE
      </span>
    </div>

    <div class="statusbar-right">
      <span v-if="activeTab" class="statusbar-item">
        Ln {{ store.state.cursorLine }}, Col {{ store.state.cursorCol }}
      </span>
      <span v-if="activeTab" class="statusbar-item">UTF-8</span>
      <span v-if="activeTab" class="statusbar-item statusbar-item--lang">{{ langLabel }}</span>
      <span v-if="activeTab?.modified" class="statusbar-item statusbar-item--modified">● Modified</span>
    </div>
  </div>
</template>

<style scoped>
.statusbar {
  height: var(--statusbar-h);
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px;
  flex-shrink: 0;
  font-family: var(--font-ui);
  font-size: 11px;
  color: rgba(255,255,255,0.9);
  user-select: none;
}

.statusbar-left,
.statusbar-right {
  display: flex;
  align-items: center;
  gap: 2px;
}

.statusbar-item {
  padding: 0 6px;
  height: 100%;
  display: flex;
  align-items: center;
  gap: 4px;
  border-radius: 3px;
  cursor: default;
  white-space: nowrap;
}
.statusbar-item:hover { background: rgba(255,255,255,0.15); }
.statusbar-item--accent { font-weight: 600; gap: 5px; }
.statusbar-item--lang { color: #fff; font-weight: 500; }
.statusbar-item--modified { color: #ffd97d; }
</style>
