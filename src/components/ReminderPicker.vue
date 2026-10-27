<script setup>
import { computed, ref, watch } from 'vue'

const props = defineProps({
  modelValue: {
    type: String,
    default: null
  }
})

const emit = defineEmits(['update:modelValue'])

const WEEKDAYS = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
const DOW = ['一', '二', '三', '四', '五', '六', '日']
const hours = Array.from({ length: 24 }, (_, index) => index)

const now = new Date()
const reminder = ref(null)
const hour = ref(9)
const minute = ref(0)
const viewYear = ref(now.getFullYear())
const viewMonth = ref(now.getMonth())

function pad(value) {
  return String(value).padStart(2, '0')
}

function parse(value) {
  if (!value) return null
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return null
  return date
}

function sameMinute(left, right) {
  return (
    left.getFullYear() === right.getFullYear() &&
    left.getMonth() === right.getMonth() &&
    left.getDate() === right.getDate() &&
    left.getHours() === right.getHours() &&
    left.getMinutes() === right.getMinutes()
  )
}

function sameDay(left, right) {
  return (
    left.getFullYear() === right.getFullYear() &&
    left.getMonth() === right.getMonth() &&
    left.getDate() === right.getDate()
  )
}

function emitDate(date) {
  const next = new Date(date)
  next.setSeconds(0, 0)
  emit('update:modelValue', next.toISOString())
}

watch(
  () => props.modelValue,
  (value) => {
    const date = parse(value)
    const previous = reminder.value
    reminder.value = date
    if (!date) return
    hour.value = date.getHours()
    minute.value = date.getMinutes()
    const dayChanged = !previous || !sameDay(previous, date)
    if (dayChanged) {
      viewYear.value = date.getFullYear()
      viewMonth.value = date.getMonth()
    }
  },
  { immediate: true }
)

const summary = computed(() => {
  const date = reminder.value
  if (!date) return '不提醒'
  return `${date.getMonth() + 1}月${date.getDate()}日 ${WEEKDAYS[date.getDay()]} ${pad(date.getHours())}:${pad(date.getMinutes())}`
})

const monthLabel = computed(() => `${viewYear.value}年${viewMonth.value + 1}月`)

const minuteOptions = computed(() => {
  const options = Array.from({ length: 12 }, (_, index) => index * 5)
  if (!options.includes(minute.value)) options.push(minute.value)
  return options.sort((left, right) => left - right)
})

const cells = computed(() => {
  const first = new Date(viewYear.value, viewMonth.value, 1)
  const offset = (first.getDay() + 6) % 7
  const start = new Date(viewYear.value, viewMonth.value, 1 - offset)
  return Array.from({ length: 42 }, (_, index) => {
    const date = new Date(start)
    date.setDate(start.getDate() + index)
    return date
  })
})

const today = new Date()

function isOutside(date) {
  return date.getMonth() !== viewMonth.value
}

function isToday(date) {
  return sameDay(date, today)
}

function isSelected(date) {
  return reminder.value ? sameDay(reminder.value, date) : false
}

function shiftMonth(delta) {
  const next = new Date(viewYear.value, viewMonth.value + delta, 1)
  viewYear.value = next.getFullYear()
  viewMonth.value = next.getMonth()
}

function selectDay(date) {
  emitDate(new Date(date.getFullYear(), date.getMonth(), date.getDate(), hour.value, minute.value, 0, 0))
}

function onHour(value) {
  hour.value = value
  const base = reminder.value ? new Date(reminder.value) : new Date()
  base.setHours(value, minute.value, 0, 0)
  emitDate(base)
}

function onMinute(value) {
  minute.value = value
  const base = reminder.value ? new Date(reminder.value) : new Date()
  base.setHours(hour.value, value, 0, 0)
  emitDate(base)
}

function clear() {
  emit('update:modelValue', null)
}

const presets = computed(() => {
  const hourLater = new Date(Date.now() + 60 * 60 * 1000)
  const tonight = new Date()
  tonight.setHours(20, 0, 0, 0)
  if (tonight.getTime() <= Date.now()) tonight.setDate(tonight.getDate() + 1)
  const morning = new Date()
  morning.setDate(morning.getDate() + 1)
  morning.setHours(9, 0, 0, 0)

  return [
    { id: 'hour', label: '一小时后', date: hourLater },
    {
      id: 'tonight',
      label: sameDay(tonight, new Date()) ? '今晚 20:00' : '明天 20:00',
      date: tonight
    },
    { id: 'morning', label: '明天 09:00', date: morning }
  ].map((item) => ({
    ...item,
    on: reminder.value ? sameMinute(reminder.value, item.date) : false
  }))
})

function applyPreset(date) {
  emitDate(date)
}

function dayName(date) {
  return `${date.getMonth() + 1}月${date.getDate()}日`
}
</script>

<template>
  <div class="pick">
    <div class="pick-label">提醒时间</div>

    <div class="pick-current">
      <p class="pick-summary" :class="{ 'is-empty': !reminder }">{{ summary }}</p>
      <button v-if="reminder" class="text-btn" type="button" @click="clear">清除提醒</button>
    </div>

    <div class="pick-shortcuts">
      <button
        v-for="preset in presets"
        :key="preset.id"
        class="pick-shortcut"
        :class="{ 'is-on': preset.on }"
        type="button"
        :aria-pressed="preset.on"
        @click="applyPreset(preset.date)"
      >
        {{ preset.label }}
      </button>
    </div>

    <div class="pick-cal">
      <div class="pick-cal-head">
        <button class="pick-nav" type="button" aria-label="上个月" @click="shiftMonth(-1)">
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M10 3.5 5.5 8 10 12.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>
        <div class="pick-month">{{ monthLabel }}</div>
        <button class="pick-nav" type="button" aria-label="下个月" @click="shiftMonth(1)">
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M6 3.5 10.5 8 6 12.5"
              fill="none"
              stroke="currentColor"
              stroke-width="1.4"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </button>
      </div>

      <div class="pick-grid">
        <span v-for="label in DOW" :key="label" class="pick-dow">{{ label }}</span>
        <button
          v-for="date in cells"
          :key="date.toDateString()"
          class="pick-day"
          :class="{
            outside: isOutside(date),
            today: isToday(date),
            selected: isSelected(date)
          }"
          type="button"
          :aria-label="dayName(date)"
          :aria-pressed="isSelected(date)"
          @click="selectDay(date)"
        >
          {{ date.getDate() }}
        </button>
      </div>
    </div>

    <div class="pick-clock" :class="{ 'is-idle': !reminder }">
      <select
        class="pick-select"
        aria-label="小时"
        :value="String(hour)"
        @change="onHour(Number($event.target.value))"
      >
        <option v-for="value in hours" :key="value" :value="String(value)">{{ pad(value) }}</option>
      </select>
      <span class="pick-colon" aria-hidden="true">:</span>
      <select
        class="pick-select"
        aria-label="分钟"
        :value="String(minute)"
        @change="onMinute(Number($event.target.value))"
      >
        <option v-for="value in minuteOptions" :key="value" :value="String(value)">{{ pad(value) }}</option>
      </select>
    </div>
  </div>
</template>
