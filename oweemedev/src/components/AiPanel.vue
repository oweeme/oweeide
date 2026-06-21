<script setup lang="ts">
import { ref, nextTick, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useEditorStore } from '../composables/useEditorStore'

interface Message {
  role: 'user' | 'assistant' | 'system'
  content: string
}

const store = useEditorStore()
const messages = ref<Message[]>([])
const input = ref('')
const isLoading = ref(false)
const error = ref('')
const messagesEl = ref<HTMLElement | null>(null)

// Settings
const showSettings = ref(false)
type Provider = 'deepseek' | 'claude' | 'openai' | 'gemini' | 'ollama'

const provider = ref<Provider>(
  (localStorage.getItem('ai_provider') as Provider) ?? 'deepseek'
)

function loadKeyForProvider(p: Provider): string {
  if (p === 'ollama') return 'ollama'
  return localStorage.getItem(`ai_key_${p}`) ?? localStorage.getItem('ai_key') ?? ''
}

const apiKey = ref(loadKeyForProvider(provider.value))
const model = ref(localStorage.getItem('ai_model') ?? 'deepseek-coder')
const ollamaModels = ref<string[]>([])
const ollamaLoading = ref(false)
const ollamaError = ref('')

interface ProviderInfo {
  label: string
  badge: string
  color: string
  keyLabel: string
  keyPlaceholder: string
  keyUrl: string
  keyNote: string
  models: { id: string; label: string }[]
}

const PROVIDERS: Record<Provider, ProviderInfo> = {
  deepseek: {
    label: 'DeepSeek', badge: '◈ DeepSeek', color: '#4d9de0',
    keyLabel: 'API Key (platform.deepseek.com)',
    keyPlaceholder: 'sk-...',
    keyUrl: 'https://platform.deepseek.com/api_keys',
    keyNote: 'API muy económica. $0.14/M tokens para DeepSeek-Coder. Obtén tu key en platform.deepseek.com.',
    models: [
      { id: 'deepseek-coder', label: 'DeepSeek Coder (Para código)' },
      { id: 'deepseek-chat',  label: 'DeepSeek Chat (General)' },
      { id: 'deepseek-reasoner', label: 'DeepSeek Reasoner (Razonamiento)' },
    ],
  },
  gemini: {
    label: 'Google Gemini', badge: '◈ Gemini', color: '#4285f4',
    keyLabel: 'API Key de aistudio.google.com (≠ Google AI Plus)',
    keyPlaceholder: 'AIza...',
    keyUrl: 'https://aistudio.google.com/app/apikey',
    keyNote: '⚠ Tu plan Google AI Plus es para la app de Gemini, NO para la API. Ve a aistudio.google.com → Get API key. Es GRATIS por separado. Alternativa: usa el CLI "gemini" en la terminal del IDE.',
    models: [
      { id: 'gemini-2.0-flash',        label: 'Gemini 2.0 Flash (Recomendado, Gratis)' },
      { id: 'gemini-2.0-flash-lite',   label: 'Gemini 2.0 Flash Lite (Rápido, Gratis)' },
      { id: 'gemini-1.5-flash-latest', label: 'Gemini 1.5 Flash Latest' },
      { id: 'gemini-1.5-pro-latest',   label: 'Gemini 1.5 Pro Latest' },
    ],
  },
  ollama: {
    label: 'Ollama (Local)', badge: '⬡ Ollama', color: '#a6e3a1',
    keyLabel: 'Sin API key — corre localmente',
    keyPlaceholder: '(no requerida)',
    keyUrl: 'https://ollama.com',
    keyNote: 'Instala Ollama y ejecuta: ollama pull llama3. 100% privado y gratis.',
    models: [
      { id: 'llama3',      label: 'Llama 3 (Recomendado)' },
      { id: 'codellama',   label: 'CodeLlama (Para código)' },
      { id: 'mistral',     label: 'Mistral 7B' },
      { id: 'deepseek-coder', label: 'DeepSeek Coder' },
      { id: 'phi3',        label: 'Phi-3 Mini (Ligero)' },
    ],
  },
  claude: {
    label: 'Anthropic Claude', badge: '◆ Claude', color: '#d97706',
    keyLabel: 'API Key de console.anthropic.com (≠ Plan Pro)',
    keyPlaceholder: 'sk-ant-...',
    keyUrl: 'https://console.anthropic.com/settings/keys',
    keyNote: '⚠ Tu plan Pro de claude.ai NO incluye API. Son facturaciones separadas. Para usar Claude sin API: escribe "claude" en la terminal del IDE — el CLI usa tu suscripción Pro directamente.',
    models: [
      { id: 'claude-sonnet-4-6',        label: 'Claude Sonnet 4.6' },
      { id: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5 (Rápido)' },
      { id: 'claude-opus-4-8',           label: 'Claude Opus 4.8 (Potente)' },
    ],
  },
  openai: {
    label: 'OpenAI / ChatGPT', badge: '⬡ OpenAI', color: '#10a37f',
    keyLabel: 'API Key (platform.openai.com)',
    keyPlaceholder: 'sk-...',
    keyUrl: 'https://platform.openai.com/api-keys',
    keyNote: 'Obtén tu key en platform.openai.com. Requiere cuenta con créditos.',
    models: [
      { id: 'gpt-4o',       label: 'GPT-4o (Recomendado)' },
      { id: 'gpt-4o-mini',  label: 'GPT-4o Mini (Rápido)' },
      { id: 'gpt-4-turbo',  label: 'GPT-4 Turbo' },
    ],
  },
}

const currentProvider = computed(() => PROVIDERS[provider.value])
const modelOptions = computed(() => PROVIDERS[provider.value].models)

async function detectOllamaModels() {
  ollamaLoading.value = true
  ollamaError.value = ''
  try {
    const models = await invoke<string[]>('list_ollama_models')
    ollamaModels.value = models
    if (models.length > 0) {
      model.value = models[0]
    } else {
      ollamaError.value = 'No hay modelos instalados. Usa: ollama pull llama3'
    }
  } catch (e: any) {
    ollamaError.value = String(e)
  }
  ollamaLoading.value = false
}

function saveSettings() {
  localStorage.setItem('ai_provider', provider.value)
  if (provider.value !== 'ollama') {
    localStorage.setItem(`ai_key_${provider.value}`, apiKey.value)
  }
  localStorage.setItem('ai_model', model.value)
  showSettings.value = false
}

function onProviderChange() {
  model.value = PROVIDERS[provider.value].models[0].id
  apiKey.value = loadKeyForProvider(provider.value)
}

// Context: include current file content
function getSystemPrompt(): string {
  const tab = store.activeTab()
  let ctx = `You are an expert programming assistant integrated into OweemeIDE.
You help with: PHP, Go, Vue, JavaScript, TypeScript, Python, Rust, SQL, HTML, CSS.
Be concise, practical, and provide code examples when useful.`
  if (tab?.type === 'code') {
    ctx += `\n\nCurrent file: ${tab.name} (${tab.language})\n\`\`\`${tab.language}\n${tab.content.slice(0, 3000)}\n\`\`\``
  }
  return ctx
}

async function sendMessage() {
  const text = input.value.trim()
  if (!text || isLoading.value) return
  if (!apiKey.value) { showSettings.value = true; error.value = 'Agrega tu API key en configuración ⚙'; return }

  input.value = ''
  error.value = ''
  messages.value.push({ role: 'user', content: text })
  isLoading.value = true
  await scrollDown()

  try {
    // Calls go through Rust (reqwest) to avoid CORS restrictions
    const history = messages.value.filter(m => m.role !== 'system').slice(-10)
    const reply = await invoke<string>('call_ai', {
      provider: provider.value,
      apiKey: apiKey.value,
      model: model.value,
      system: getSystemPrompt(),
      messages: history,
    })
    messages.value.push({ role: 'assistant', content: reply })
  } catch (e: any) {
    error.value = String(e)
    messages.value.push({ role: 'assistant', content: `❌ ${String(e)}` })
  }

  isLoading.value = false
  await scrollDown()
}

async function scrollDown() {
  await nextTick()
  if (messagesEl.value) messagesEl.value.scrollTop = messagesEl.value.scrollHeight
}

function clearChat() { messages.value = [] }

function insertCode(content: string) {
  // Extract code blocks and insert into active editor
  const codeMatch = content.match(/```[\w]*\n?([\s\S]+?)```/)
  const code = codeMatch ? codeMatch[1] : content
  const tab = store.activeTab()
  if (tab?.type === 'code') {
    store.updateContent(tab.path, tab.content + '\n' + code)
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage() }
}

// Markdown-like rendering (simple)
function renderContent(text: string): string {
  return text
    .replace(/```(\w*)\n?([\s\S]*?)```/g, (_, lang, code) =>
      `<pre class="ai-code-block"><code class="lang-${lang}">${escapeHtml(code.trim())}</code></pre>`)
    .replace(/`([^`]+)`/g, '<code class="ai-inline-code">$1</code>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/\n/g, '<br>')
}

function escapeHtml(s: string) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')
}
</script>

<template>
  <div class="ai-panel">
    <!-- Header -->
    <div class="ai-header">
      <div class="ai-header-left">
        <span class="ai-logo">✦</span>
        <span class="ai-title">AI Assistant</span>
        <span class="ai-provider-badge" :style="{ color: currentProvider.color }">{{ currentProvider.badge }}</span>
      </div>
      <div class="ai-header-right">
        <button class="ai-hdr-btn" @click="clearChat" title="Clear chat">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
            <path d="M5.5 5.5A.5.5 0 016 6v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm2.5 0a.5.5 0 01.5.5v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm3 .5a.5.5 0 00-1 0v6a.5.5 0 001 0V6z"/><path fill-rule="evenodd" d="M14.5 3a1 1 0 01-1 1H13v9a2 2 0 01-2 2H5a2 2 0 01-2-2V4h-.5a1 1 0 01-1-1V2a1 1 0 011-1H6a1 1 0 011-1h2a1 1 0 011 1h3.5a1 1 0 011 1v1zM4.118 4L4 4.059V13a1 1 0 001 1h6a1 1 0 001-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z"/>
          </svg>
        </button>
        <button class="ai-hdr-btn" @click="showSettings = !showSettings" title="Settings">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
            <path d="M9.405 1.05c-.413-1.4-2.397-1.4-2.81 0l-.1.34a1.464 1.464 0 01-2.105.872l-.31-.17c-1.283-.698-2.686.705-1.987 1.987l.169.311c.446.82.023 1.841-.872 2.105l-.34.1c-1.4.413-1.4 2.397 0 2.81l.34.1a1.464 1.464 0 01.872 2.105l-.17.31c-.698 1.283.705 2.686 1.987 1.987l.311-.169a1.464 1.464 0 012.105.872l.1.34c.413 1.4 2.397 1.4 2.81 0l.1-.34a1.464 1.464 0 012.105-.872l.31.17c1.283.698 2.686-.705 1.987-1.987l-.169-.311a1.464 1.464 0 01.872-2.105l.34-.1c1.4-.413 1.4-2.397 0-2.81l-.34-.1a1.464 1.464 0 01-.872-2.105l.17-.31c.698-1.283-.705-2.686-1.987-1.987l-.311.169a1.464 1.464 0 01-2.105-.872l-.1-.34zM8 10.93a2.929 2.929 0 110-5.858 2.929 2.929 0 010 5.858z"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Settings panel -->
    <transition name="slide">
      <div v-if="showSettings" class="ai-settings">
        <h3 class="settings-title">AI Settings</h3>

        <!-- Provider selector grid -->
        <div class="settings-field">
          <label>Provider</label>
          <div class="provider-grid">
            <label
              v-for="(info, key) in PROVIDERS"
              :key="key"
              class="provider-opt"
              :class="{ active: provider === key }"
              :style="provider === key ? { borderColor: info.color, color: info.color, background: info.color + '15' } : {}"
            >
              <input type="radio" v-model="provider" :value="key" @change="onProviderChange" hidden />
              <span class="provider-badge-sm">{{ info.badge }}</span>
              <span class="provider-name">{{ info.label }}</span>
            </label>
          </div>
        </div>

        <!-- Model selector -->
        <div class="settings-field">
          <label>Model</label>
          <!-- Ollama: detect locally installed models -->
          <div v-if="provider === 'ollama'" class="ollama-model-row">
            <select v-model="model" class="settings-select" style="flex:1">
              <option v-if="ollamaModels.length === 0" v-for="m in modelOptions" :key="m.id" :value="m.id">{{ m.label }}</option>
              <option v-for="m in ollamaModels" :key="m" :value="m">{{ m }}</option>
            </select>
            <button class="ollama-detect-btn" @click="detectOllamaModels" :disabled="ollamaLoading">
              {{ ollamaLoading ? '…' : '⟳ Detectar' }}
            </button>
          </div>
          <select v-else v-model="model" class="settings-select">
            <option v-for="m in modelOptions" :key="m.id" :value="m.id">{{ m.label }}</option>
          </select>
          <div v-if="ollamaError" class="ollama-error">{{ ollamaError }}</div>
        </div>

        <!-- API Key (hidden for Ollama) -->
        <div class="settings-field" v-if="provider !== 'ollama'">
          <label>{{ currentProvider.keyLabel }}</label>
          <input
            v-model="apiKey"
            type="password"
            class="settings-input"
            :placeholder="currentProvider.keyPlaceholder"
          />
          <a class="settings-link" :href="currentProvider.keyUrl" target="_blank">
            → Obtener API key
          </a>
        </div>

        <!-- Provider-specific note -->
        <div class="settings-note" :class="provider === 'claude' ? 'note-warn' : ''">
          {{ currentProvider.keyNote }}
          <template v-if="provider === 'claude'">
            <br><br>
            <strong>Tip:</strong> En la terminal del IDE escribe <code>claude</code> para usar tu plan Pro sin API key.
          </template>
          <template v-if="provider === 'gemini'">
            <br><br>
            <strong>Tip:</strong> En la terminal del IDE escribe <code>gemini</code> para usar tu cuenta sin API key.
          </template>
          <template v-if="provider === 'ollama'">
            <br><br>
            <code>ollama pull llama3</code> para descargar el modelo.
          </template>
        </div>

        <button class="settings-save" @click="saveSettings">Guardar y Cerrar</button>
      </div>
    </transition>

    <!-- Messages -->
    <div ref="messagesEl" class="ai-messages">
      <div v-if="messages.length === 0" class="ai-empty">
        <div class="ai-empty-icon">✦</div>
        <p>Ask anything about your code</p>
        <div class="ai-suggestions">
          <button class="ai-suggestion" @click="input = 'Explain this file'; sendMessage()">Explain this file</button>
          <button class="ai-suggestion" @click="input = 'Find bugs in the current code'; sendMessage()">Find bugs</button>
          <button class="ai-suggestion" @click="input = 'How can I optimize this?'; sendMessage()">Optimize code</button>
          <button class="ai-suggestion" @click="input = 'Write unit tests for this'; sendMessage()">Write tests</button>
        </div>
      </div>

      <div
        v-for="(msg, i) in messages"
        :key="i"
        class="ai-msg"
        :class="msg.role === 'user' ? 'ai-msg--user' : 'ai-msg--assistant'"
      >
        <div class="ai-msg-avatar">
          {{ msg.role === 'user' ? 'U' : '✦' }}
        </div>
        <div class="ai-msg-content">
          <div v-if="msg.role === 'assistant'" v-html="renderContent(msg.content)" class="ai-rendered" />
          <div v-else class="ai-user-text">{{ msg.content }}</div>
          <button
            v-if="msg.role === 'assistant' && msg.content.includes('```')"
            class="ai-insert-btn"
            @click="insertCode(msg.content)"
            title="Insert code into editor"
          >
            ↗ Insert code
          </button>
        </div>
      </div>

      <div v-if="isLoading" class="ai-msg ai-msg--assistant">
        <div class="ai-msg-avatar">✦</div>
        <div class="ai-typing">
          <span /><span /><span />
        </div>
      </div>
    </div>

    <!-- Input -->
    <div class="ai-input-area">
      <div v-if="error && !isLoading" class="ai-error">{{ error }}</div>
      <div class="ai-input-row">
        <textarea
          v-model="input"
          class="ai-textarea"
          placeholder="Ask about your code… (Enter to send, Shift+Enter for newline)"
          rows="2"
          @keydown="onKeydown"
        />
        <button
          class="ai-send-btn"
          :disabled="isLoading || !input.trim()"
          @click="sendMessage"
        >
          <svg v-if="!isLoading" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M15.854.146a.5.5 0 01.11.54l-5.819 14.547a.75.75 0 01-1.329.124l-3.178-4.995L.643 7.184a.75.75 0 01.124-1.33L15.314.037a.5.5 0 01.54.11z"/>
          </svg>
          <span v-else class="send-spinner" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ai-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-dark);
  font-family: var(--font-ui);
  overflow: hidden;
}

/* Header */
.ai-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  background: var(--bg-darkest);
}
.ai-header-left { display: flex; align-items: center; gap: 8px; }
.ai-logo { font-size: 16px; color: var(--accent); }
.ai-title { font-size: 13px; font-weight: 600; color: var(--fg-bright); }
.ai-provider-badge {
  font-size: 10px; font-weight: 600;
  background: rgba(82,139,255,0.15);
  border: 1px solid rgba(82,139,255,0.3);
  color: var(--accent);
  border-radius: 10px; padding: 1px 7px;
}
.ai-header-right { display: flex; gap: 2px; }
.ai-hdr-btn {
  width: 26px; height: 26px;
  background: none; border: none; border-radius: 5px;
  color: var(--fg-muted); cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: background 0.12s, color 0.12s;
}
.ai-hdr-btn:hover { background: var(--bg-hover); color: var(--fg); }

/* Settings */
.ai-settings {
  background: var(--bg-mid);
  border-bottom: 1px solid var(--border);
  padding: 14px;
  flex-shrink: 0;
}
.settings-title { font-size: 12px; font-weight: 700; color: var(--fg); margin-bottom: 12px; }
.settings-field { margin-bottom: 10px; }
.settings-field label { display: block; font-size: 11px; color: var(--fg-muted); margin-bottom: 4px; }
.provider-grid {
  display: grid; grid-template-columns: 1fr 1fr;
  gap: 5px;
}
.provider-opt {
  border: 1px solid var(--border); border-radius: 7px;
  padding: 7px 8px; cursor: pointer;
  display: flex; flex-direction: column; gap: 2px;
  transition: all 0.12s; color: var(--fg-dim);
}
.provider-opt:hover { background: var(--bg-hover); }
.provider-badge-sm { font-size: 12px; font-weight: 700; }
.provider-name { font-size: 10px; }
.settings-link {
  display: inline-block; margin-top: 4px;
  font-size: 10.5px; color: var(--accent);
  text-decoration: none;
}
.settings-link:hover { text-decoration: underline; }
.note-warn { border-left: 2px solid #d97706; padding-left: 8px; }
.ollama-model-row { display: flex; gap: 6px; align-items: center; }
.ollama-detect-btn {
  background: var(--bg-hover); border: 1px solid var(--border);
  border-radius: 5px; color: var(--accent); font-size: 11px;
  padding: 4px 8px; cursor: pointer; white-space: nowrap;
  transition: all 0.12s;
}
.ollama-detect-btn:hover { background: var(--bg-active); }
.ollama-detect-btn:disabled { opacity: 0.5; cursor: default; }
.ollama-error { font-size: 10px; color: var(--red, #f85149); margin-top: 4px; }
.settings-select, .settings-input {
  width: 100%; background: var(--bg-darkest);
  border: 1px solid var(--border); border-radius: 5px;
  color: var(--fg); font-size: 12px; padding: 5px 8px;
  outline: none; font-family: var(--font-mono);
}
.settings-select:focus, .settings-input:focus { border-color: var(--accent); }
.settings-note { font-size: 10px; color: var(--fg-muted); margin: 8px 0; line-height: 1.4; }
.settings-save {
  background: var(--accent); border: none; border-radius: 6px;
  color: #fff; font-size: 12px; font-weight: 600;
  padding: 6px 16px; cursor: pointer; width: 100%;
}
.settings-save:hover { opacity: 0.85; }

/* Messages */
.ai-messages {
  flex: 1;
  overflow-y: auto;
  padding: 12px 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.ai-messages::-webkit-scrollbar { width: 4px; }
.ai-messages::-webkit-scrollbar-thumb { background: var(--bg-active); border-radius: 2px; }

.ai-empty {
  flex: 1; display: flex; flex-direction: column;
  align-items: center; justify-content: center;
  gap: 10px; padding: 20px; text-align: center;
}
.ai-empty-icon { font-size: 32px; color: var(--accent); opacity: 0.5; }
.ai-empty p { font-size: 12px; color: var(--fg-muted); }
.ai-suggestions { display: flex; flex-direction: column; gap: 5px; width: 100%; }
.ai-suggestion {
  background: var(--bg-mid); border: 1px solid var(--border);
  color: var(--fg-dim); border-radius: 6px; padding: 6px 10px;
  font-size: 11.5px; cursor: pointer; text-align: left;
  transition: all 0.12s;
}
.ai-suggestion:hover { background: var(--bg-hover); color: var(--fg); border-color: var(--accent); }

.ai-msg {
  display: flex; gap: 10px; padding: 6px 14px;
  align-items: flex-start;
}
.ai-msg--user { flex-direction: row-reverse; }
.ai-msg-avatar {
  width: 26px; height: 26px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 11px; font-weight: 700; flex-shrink: 0;
  background: var(--bg-active); color: var(--fg-dim);
}
.ai-msg--user .ai-msg-avatar { background: var(--accent); color: #fff; }
.ai-msg--assistant .ai-msg-avatar { background: rgba(82,139,255,0.15); color: var(--accent); }

.ai-msg-content { flex: 1; min-width: 0; }
.ai-msg--user .ai-msg-content { display: flex; justify-content: flex-end; }
.ai-user-text {
  background: var(--accent); color: #fff;
  border-radius: 12px 12px 2px 12px;
  padding: 8px 12px; font-size: 12.5px; line-height: 1.5;
  max-width: 90%; word-break: break-word;
}
.ai-rendered {
  font-size: 12.5px; color: var(--fg); line-height: 1.6;
  word-break: break-word;
}

.ai-insert-btn {
  margin-top: 6px; background: var(--bg-hover);
  border: 1px solid var(--border); border-radius: 5px;
  color: var(--accent); font-size: 11px; padding: 3px 8px;
  cursor: pointer; transition: all 0.12s;
}
.ai-insert-btn:hover { background: var(--bg-active); }

.ai-typing {
  display: flex; gap: 4px; padding: 10px 0; align-items: center;
}
.ai-typing span {
  width: 6px; height: 6px; border-radius: 50%;
  background: var(--fg-muted); animation: blink 1.2s infinite;
}
.ai-typing span:nth-child(2) { animation-delay: 0.2s; }
.ai-typing span:nth-child(3) { animation-delay: 0.4s; }
@keyframes blink { 0%,60%,100% { opacity:0.2 } 30% { opacity:1 } }

/* Input */
.ai-input-area {
  border-top: 1px solid var(--border);
  padding: 8px;
  flex-shrink: 0;
  background: var(--bg-darkest);
}
.ai-error { font-size: 11px; color: var(--red); padding: 0 4px 6px; }
.ai-input-row { display: flex; gap: 6px; align-items: flex-end; }
.ai-textarea {
  flex: 1; background: var(--bg-mid);
  border: 1px solid var(--border); border-radius: 8px;
  color: var(--fg); font-size: 12.5px; line-height: 1.5;
  padding: 8px 10px; resize: none; outline: none;
  font-family: var(--font-ui); min-height: 38px;
  transition: border-color 0.12s;
}
.ai-textarea:focus { border-color: var(--accent); }
.ai-send-btn {
  width: 36px; height: 36px;
  background: var(--accent); border: none; border-radius: 8px;
  color: #fff; cursor: pointer; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  transition: opacity 0.12s;
}
.ai-send-btn:hover { opacity: 0.85; }
.ai-send-btn:disabled { opacity: 0.4; cursor: default; }
.send-spinner {
  width: 14px; height: 14px; border: 2px solid rgba(255,255,255,0.3);
  border-top-color: #fff; border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }

.slide-enter-active, .slide-leave-active { transition: all 0.18s ease; }
.slide-enter-from, .slide-leave-to { opacity: 0; transform: translateY(-6px); }
</style>

<style>
/* Global for v-html rendered content */
.ai-rendered .ai-code-block {
  background: #0d0e10;
  border: 1px solid #2d2d32;
  border-radius: 6px;
  padding: 10px 12px;
  margin: 8px 0;
  overflow-x: auto;
  font-family: 'JetBrains Mono', 'Fira Code', Consolas, monospace;
  font-size: 12px;
  line-height: 1.5;
  color: #cdd6f4;
  white-space: pre;
}
.ai-rendered .ai-inline-code {
  background: rgba(82,139,255,0.1);
  border: 1px solid rgba(82,139,255,0.2);
  border-radius: 3px;
  padding: 1px 5px;
  font-family: 'JetBrains Mono', Consolas, monospace;
  font-size: 11.5px;
  color: #89b4fa;
}
</style>
