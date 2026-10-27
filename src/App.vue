<script setup>
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import TodoItem from './components/TodoItem.vue'
import TodoEditor from './components/TodoEditor.vue'
import SettingsDialog from './components/SettingsDialog.vue'

const todos = ref([])
const filter = ref('active')
const draft = ref('')
const editing = ref(null)
const settingsOpen = ref(false)
const autoLaunch = ref(true)
const loaded = ref(false)
const theme = ref(document.documentElement.dataset.theme === 'light' ? 'light' : 'dark')

const filteredTodos = computed(() => {
  const list = [...todos.value].sort((a, b) => {
    if (a.completed !== b.completed) return a.completed ? 1 : -1
    return (b.updatedAt || b.createdAt) - (a.updatedAt || a.createdAt)
  })

  if (filter.value === 'active') return list.filter((t) => !t.completed)
  if (filter.value === 'done') return list.filter((t) => t.completed)
  return list
})

const remaining = computed(() => todos.value.filter((t) => !t.completed).length)

const todayLabel = computed(() => {
  const now = new Date()
  const date = now.toLocaleDateString('zh-CN', { month: 'long', day: 'numeric' })
  const weekday = now.toLocaleDateString('zh-CN', { weekday: 'short' })
  return `${date} ${weekday}`
})

const emptyText = computed(() => {
  if (filter.value === 'done') return '还没有已完成的事项'
  if (filter.value === 'all') return '还没有待办。在上面写下第一件'
  return '还没有进行中的事项。在上面写下要做的事'
})

async function persist() {
  await invoke('save_todos', { todos: todos.value })
}

function createId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}

async function addTodo() {
  const title = draft.value.trim()
  if (!title) return

  const now = Date.now()
  todos.value.unshift({
    id: createId(),
    title,
    completed: false,
    reminderAt: null,
    reminded: false,
    createdAt: now,
    updatedAt: now
  })
  draft.value = ''
  await persist()
}

function openEdit(todo) {
  editing.value = { ...todo }
}

async function saveEdit(payload) {
  const idx = todos.value.findIndex((t) => t.id === payload.id)
  if (idx === -1) {
    editing.value = null
    return
  }

  const prev = todos.value[idx]
  const reminderChanged = prev.reminderAt !== payload.reminderAt
  todos.value[idx] = {
    ...prev,
    title: payload.title.trim(),
    reminderAt: payload.reminderAt || null,
    reminded: reminderChanged ? false : prev.reminded,
    updatedAt: Date.now()
  }
  editing.value = null
  await persist()
}

async function toggleComplete(todo) {
  const idx = todos.value.findIndex((t) => t.id === todo.id)
  if (idx === -1) return
  todos.value[idx] = {
    ...todos.value[idx],
    completed: !todos.value[idx].completed,
    updatedAt: Date.now()
  }
  await persist()
}

async function removeTodo(todo) {
  todos.value = todos.value.filter((t) => t.id !== todo.id)
  await persist()
}

async function clearCompleted() {
  todos.value = todos.value.filter((t) => !t.completed)
  await persist()
}

async function toggleAutoLaunch() {
  autoLaunch.value = await invoke('set_autolaunch', { enabled: !autoLaunch.value })
}

function applyTodos(next) {
  todos.value = next
  editing.value = null
}

function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
  document.documentElement.dataset.theme = theme.value
  localStorage.setItem('desktop-todo-theme', theme.value)
}

onMounted(async () => {
  todos.value = await invoke('get_todos')
  autoLaunch.value = await invoke('get_autolaunch')
  loaded.value = true
})
</script>

<template>
  <div class="app-shell">
    <div class="panel">
      <header class="titlebar">
        <h1 data-tauri-drag-region>待办事项</h1>
        <div class="titlebar-drag" data-tauri-drag-region></div>
        <span class="today" data-tauri-drag-region>{{ todayLabel }}</span>
        <div class="titlebar-tools">
          <button class="gear-btn" type="button" aria-label="设置" title="设置" @click="settingsOpen = true">
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path
                fill="none"
                stroke="currentColor"
                stroke-width="1.3"
                stroke-linejoin="round"
                d="M6.7 1.8h2.6l.3 1.6a4.6 4.6 0 0 1 1.5.9l1.5-.7 1.3 2.2-1.2 1.1a4.6 4.6 0 0 1 0 1.8l1.2 1.1-1.3 2.2-1.5-.7a4.6 4.6 0 0 1-1.5.9l-.3 1.6H6.7l-.3-1.6a4.6 4.6 0 0 1-1.5-.9l-1.5.7-1.3-2.2 1.2-1.1a4.6 4.6 0 0 1 0-1.8L2.1 5.8l1.3-2.2 1.5.7a4.6 4.6 0 0 1 1.5-.9l.3-1.6Z"
              />
              <circle cx="8" cy="8" r="1.7" fill="none" stroke="currentColor" stroke-width="1.3" />
            </svg>
          </button>
          <button
            class="theme-btn"
            type="button"
            :aria-label="theme === 'dark' ? '切换为亮色' : '切换为暗色'"
            @click="toggleTheme"
          >
            {{ theme === 'dark' ? '亮色' : '暗色' }}
          </button>
        </div>
      </header>

      <div class="toolbar">
        <input
          v-model="draft"
          type="text"
          maxlength="200"
          placeholder="写下要做的事，回车添加"
          @keydown.enter="addTodo"
        />
        <!-- <button class="primary-btn" type="button" :disabled="!draft.trim()" @click="addTodo">添加</button> -->
      </div>

      <div class="filters">
        <button class="chip" type="button" :class="{ active: filter === 'active' }" @click="filter = 'active'">
          进行中
        </button>
        <button class="chip" type="button" :class="{ active: filter === 'all' }" @click="filter = 'all'">
          全部
        </button>
        <button class="chip" type="button" :class="{ active: filter === 'done' }" @click="filter = 'done'">
          已完成
        </button>
      </div>

      <div class="list">
        <p v-if="loaded && !filteredTodos.length" class="empty">{{ emptyText }}</p>

        <TodoItem
          v-for="todo in filteredTodos"
          :key="todo.id"
          :todo="todo"
          @toggle="toggleComplete"
          @edit="openEdit"
          @remove="removeTodo"
        />
      </div>

      <footer class="footer">
        <span class="remaining">{{ remaining }} 项未完成</span>
        <div class="footer-actions">
          <label class="toggle">
            <input type="checkbox" :checked="autoLaunch" @change="toggleAutoLaunch" />
            <span class="toggle-mark" aria-hidden="true"></span>
            开机自启
          </label>
          <button class="text-btn" type="button" @click="clearCompleted">清理已完成</button>
        </div>
      </footer>
    </div>

    <TodoEditor
      v-if="editing"
      :todo="editing"
      @cancel="editing = null"
      @save="saveEdit"
    />

    <SettingsDialog v-if="settingsOpen" @close="settingsOpen = false" @changed="applyTodos" />
  </div>
</template>
