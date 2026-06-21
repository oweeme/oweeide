<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()

function minimize() { appWindow.minimize() }
function maximize() { appWindow.toggleMaximize() }
function close() { appWindow.close() }
</script>

<template>
  <div class="titlebar" data-tauri-drag-region>
    <div class="titlebar-logo" data-tauri-drag-region>
      <img src="/oweedev.png" width="18" height="18" style="border-radius:4px;display:block;" />
      <span class="titlebar-name">OweemeIDE</span>
    </div>
    <div class="titlebar-spacer" data-tauri-drag-region />
    <div class="titlebar-controls">
      <button class="tb-btn tb-minimize" @click="minimize" title="Minimize">
        <svg width="10" height="10" viewBox="0 0 10 10"><rect y="4.5" width="10" height="1" fill="currentColor"/></svg>
      </button>
      <button class="tb-btn tb-maximize" @click="maximize" title="Maximize">
        <svg width="10" height="10" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" stroke="currentColor" fill="none"/></svg>
      </button>
      <button class="tb-btn tb-close" @click="close" title="Close">
        <svg width="10" height="10" viewBox="0 0 10 10">
          <line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" stroke-width="1.3"/>
          <line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" stroke-width="1.3"/>
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  height: 30px;
  background: linear-gradient(90deg, #152e29 0%, #1a3d35 40%, #152e29 100%);
  border-bottom: 1px solid #0e201c;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  user-select: none;
  -webkit-user-select: none;
}

.titlebar-logo {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 12px;
}

.titlebar-name {
  font-size: 12px;
  font-weight: 700;
  color: #c8ddd8;
  letter-spacing: 0.4px;
  font-family: 'Segoe UI', system-ui, sans-serif;
}

.titlebar-spacer {
  flex: 1;
}

.titlebar-controls {
  display: flex;
  align-items: stretch;
  height: 100%;
}

.tb-btn {
  width: 42px;
  height: 100%;
  background: none;
  border: none;
  color: #8ab3aa;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.12s, color 0.12s;
}

.tb-btn:hover { background: rgba(255,255,255,0.08); color: #c8ddd8; }
.tb-close:hover { background: #d94f30 !important; color: #fff !important; }
</style>
