<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { cachedAvatarImageUrl } from '../../utils/avatarImageCache'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  alt: string
  decoding?: 'async' | 'auto' | 'sync'
  loading?: 'eager' | 'lazy'
  src?: string | null
}>(), {
  decoding: 'async',
  loading: 'lazy',
  src: '',
})

const emit = defineEmits<{
  error: [event: Event]
}>()

const displaySrc = ref('')
const fallbackAttempted = ref(false)
const sourceUrl = computed(() => String(props.src ?? '').trim())

watch(sourceUrl, async (value, _previousValue, onCleanup) => {
  let active = true
  onCleanup(() => {
    active = false
  })

  displaySrc.value = ''
  fallbackAttempted.value = false
  if (!value) return

  const cachedUrl = await cachedAvatarImageUrl(value)
  if (!active) return
  displaySrc.value = cachedUrl
}, { immediate: true })

function handleError(event: Event) {
  if (!fallbackAttempted.value && sourceUrl.value && displaySrc.value !== sourceUrl.value) {
    fallbackAttempted.value = true
    displaySrc.value = sourceUrl.value
    return
  }

  displaySrc.value = ''
  emit('error', event)
}
</script>

<template>
  <img
    v-if="displaySrc"
    v-bind="$attrs"
    :alt="alt"
    :decoding="decoding"
    :loading="loading"
    :src="displaySrc"
    @error="handleError"
  />
  <slot v-else></slot>
</template>
