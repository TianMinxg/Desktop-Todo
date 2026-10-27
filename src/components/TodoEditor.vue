<script setup>
import { computed, ref, watch } from 'vue'
import ReminderPicker from './ReminderPicker.vue'

const props = defineProps({
  todo: {
    type: Object,
    required: true
  }
})

const emit = defineEmits(['save', 'cancel'])

const title = ref(props.todo.title || '')
const reminderAt = ref(props.todo.reminderAt || null)

watch(
  () => props.todo,
  (todo) => {
    title.value = todo.title || ''
    reminderAt.value = todo.reminderAt || null
  },
  { immediate: true }
)

const canSave = computed(() => title.value.trim().length > 0)

function sameMinute(left, right) {
  if (!left && !right) return true
  if (!left || !right) return false
  const a = new Date(left)
  const b = new Date(right)
  if (Number.isNaN(a.getTime()) || Number.isNaN(b.getTime())) return false
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate() &&
    a.getHours() === b.getHours() &&
    a.getMinutes() === b.getMinutes()
  )
}

function submit() {
  if (!canSave.value) return
  emit('save', {
    id: props.todo.id,
    title: title.value,
    reminderAt: sameMinute(props.todo.reminderAt, reminderAt.value)
      ? props.todo.reminderAt || null
      : reminderAt.value
  })
}
</script>

<template>
  <div class="modal-mask" @mousedown.self="emit('cancel')">
    <div class="modal">
      <h2>编辑待办</h2>

      <div class="field">
        <textarea v-model="title" rows="2" maxlength="200" placeholder="待办内容" />
      </div>

      <div class="field">
        <ReminderPicker v-model="reminderAt" />
      </div>

      <div class="modal-actions">
        <button class="ghost-btn" type="button" @click="emit('cancel')">取消</button>
        <button class="primary-btn" type="button" :disabled="!canSave" @click="submit">保存</button>
      </div>
    </div>
  </div>
</template>
