<script setup>
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const emit = defineEmits(['close', 'changed'])

const dataDir = ref('')
const error = ref('')
const saving = ref(false)

onMounted(async () => {
  try {
    dataDir.value = await invoke('get_data_dir')
  } catch {
    error.value = '无法读取当前数据目录'
  }
})

function messageOf(err) {
  if (typeof err === 'string' && err) return err
  return '无法保存这个目录'
}

async function browse() {
  error.value = ''
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择数据目录',
      defaultPath: dataDir.value || undefined
    })
    if (typeof selected === 'string' && selected) {
      dataDir.value = selected
    }
  } catch {
    error.value = '无法打开文件夹选择'
  }
}

async function apply(path) {
  error.value = ''
  saving.value = true
  try {
    const result = await invoke('set_data_dir', { path })
    emit('changed', result.todos)
    emit('close')
  } catch (err) {
    error.value = messageOf(err)
  } finally {
    saving.value = false
  }
}

function save() {
  if (!dataDir.value.trim()) {
    error.value = '请填写数据目录'
    return
  }
  apply(dataDir.value.trim())
}

function resetDefault() {
  apply('')
}
</script>

<template>
  <div class="modal-mask settings-mask" @mousedown.self="emit('close')">
    <div class="modal" role="dialog" aria-labelledby="settings-title">
      <h2 id="settings-title">设置</h2>

      <section class="field">
        <label for="data-dir">数据目录</label>
        <div class="settings-path">
          <input
            id="data-dir"
            v-model="dataDir"
            type="text"
            spellcheck="false"
            placeholder="例如 D:\Todos"
            :title="dataDir"
            :disabled="saving"
            @keydown.enter="save"
          />
          <button class="ghost-btn" type="button" :disabled="saving" @click="browse">浏览</button>
        </div>
        <p class="field-hint">
          待办写在该文件夹的 todos.json。换到空文件夹时会把现有数据移过去；新文件夹里已有这份文件时，改用那份数据。
        </p>
        <p v-if="error" class="field-error">{{ error }}</p>
      </section>

      <div class="modal-actions settings-actions">
        <button class="text-btn" type="button" :disabled="saving" @click="resetDefault">恢复默认</button>
        <div class="settings-actions-end">
          <button class="ghost-btn" type="button" :disabled="saving" @click="emit('close')">取消</button>
          <button class="primary-btn" type="button" :disabled="saving" @click="save">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>
