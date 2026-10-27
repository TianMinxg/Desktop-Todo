<script setup>
import { computed } from 'vue'

const props = defineProps({
  todo: {
    type: Object,
    required: true
  }
})

const emit = defineEmits(['toggle', 'edit', 'remove'])

const meta = computed(() => {
  if (!props.todo.reminderAt) return null
  const time = new Date(props.todo.reminderAt)
  const label = new Intl.DateTimeFormat('zh-CN', {
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23'
  }).format(time)
  const diff = time.getTime() - Date.now()
  let tone = ''
  if (!props.todo.completed) {
    if (diff < 0) tone = 'overdue'
    else if (diff < 60 * 60 * 1000) tone = 'due-soon'
  }
  return { label: `提醒 ${label}`, tone }
})
</script>

<template>
  <div class="todo-item" :class="{ completed: todo.completed }">
    <button
      class="check"
      type="button"
      :aria-pressed="todo.completed"
      aria-label="标记完成"
      title="完成或恢复"
      @click="emit('toggle', todo)"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M3.8 8.2 6.5 10.8 12.2 5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>

    <div class="todo-main" @dblclick="emit('edit', todo)">
      <div class="todo-title">{{ todo.title }}</div>
    </div>

    <div v-if="meta" class="todo-meta" :class="meta.tone">{{ meta.label }}</div>

    <div class="todo-actions">
      <button class="icon-btn" type="button" aria-label="编辑" title="编辑" @click="emit('edit', todo)">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M9.1 3.4 12.6 6.9 6.1 13.4H2.6V9.9Z"
            fill="none"
            stroke="currentColor"
            stroke-width="1.3"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <button class="icon-btn danger" type="button" aria-label="删除" title="删除" @click="emit('remove', todo)">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M4.2 4.2 11.8 11.8M11.8 4.2 4.2 11.8"
            fill="none"
            stroke="currentColor"
            stroke-width="1.3"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>
  </div>
</template>
