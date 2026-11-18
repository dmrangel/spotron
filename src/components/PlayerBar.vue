<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { api, fmtTime, player } from "../player";

const emit = defineEmits<{ error: [msg: string] }>();

// Posição interpolada entre as atualizações da página do SDK (1 s).
const now = ref(Date.now());
let timer = 0;
onMounted(() => (timer = window.setInterval(() => (now.value = Date.now()), 250)));
onUnmounted(() => clearInterval(timer));

const st = computed(() => player.state);
const position = computed(() => {
  const s = st.value;
  if (!s) return 0;
  const p = s.paused ? s.position : s.position + (now.value - s.ts);
  return Math.min(p, s.duration);
});
const volume = ref(100);
const dragging = ref<number | null>(null);

async function run(fn: () => Promise<unknown>) {
  try {
    await fn();
  } catch (e) {
    emit("error", String(e));
  }
}

function onSeekInput(e: Event) {
  dragging.value = Number((e.target as HTMLInputElement).value);
}
function onSeekChange() {
  const ms = dragging.value;
  dragging.value = null;
  if (ms !== null) run(() => api.seek(ms));
}
</script>

<template>
  <footer class="flex h-24 items-center gap-6 border-t border-app-border bg-app-panel px-4">
    <!-- Faixa atual -->
    <div class="flex w-72 min-w-0 items-center gap-3">
      <img v-if="st?.track?.image" :src="st.track.image" class="h-14 w-14 rounded-app-sm object-cover" alt="" />
      <div v-else class="h-14 w-14 rounded-app-sm bg-app-card" />
      <div class="min-w-0">
        <div class="truncate font-semibold text-app-text-strong">{{ st?.track?.name ?? "Nada tocando" }}</div>
        <div class="truncate text-sm text-app-text-muted">{{ st?.track?.artists ?? "" }}</div>
      </div>
    </div>

    <!-- Controles e progresso -->
    <div class="flex flex-1 flex-col items-center gap-1">
      <div class="flex items-center gap-4">
        <button class="text-app-text hover:text-app-text-strong" title="Anterior" @click="run(api.prev)">
          <svg viewBox="0 0 24 24" class="h-5 w-5 fill-current"><path d="M6 5h2v14H6zM20 5v14L9 12z" /></svg>
        </button>
        <button
          class="flex h-10 w-10 items-center justify-center rounded-full bg-app-text-strong text-app-bg hover:scale-105"
          :title="st?.paused === false ? 'Pausar' : 'Tocar'"
          @click="run(st?.paused === false ? api.pause : api.resume)"
        >
          <svg v-if="st?.paused === false" viewBox="0 0 24 24" class="h-5 w-5 fill-current"><path d="M6 5h4v14H6zM14 5h4v14h-4z" /></svg>
          <svg v-else viewBox="0 0 24 24" class="h-5 w-5 fill-current"><path d="M7 5v14l12-7z" /></svg>
        </button>
        <button class="text-app-text hover:text-app-text-strong" title="Próxima" @click="run(api.next)">
          <svg viewBox="0 0 24 24" class="h-5 w-5 fill-current"><path d="M16 5h2v14h-2zM4 5v14l11-7z" /></svg>
        </button>
      </div>
      <div class="flex w-full max-w-xl items-center gap-2 text-xs text-app-text-muted">
        <span class="w-10 text-right">{{ fmtTime(dragging ?? position) }}</span>
        <input
          type="range"
          min="0"
          :max="st?.duration || 1"
          :value="dragging ?? position"
          class="flex-1 accent-[var(--app-accent)]"
          :disabled="!st?.track"
          @input="onSeekInput"
          @change="onSeekChange"
        />
        <span class="w-10">{{ fmtTime(st?.duration ?? 0) }}</span>
      </div>
    </div>

    <!-- Volume -->
    <div class="flex w-48 items-center gap-2 text-app-text-muted">
      <svg viewBox="0 0 24 24" class="h-5 w-5 fill-current"><path d="M4 9v6h4l5 4V5L8 9zm12.5 3a4.5 4.5 0 0 0-2.5-4v8a4.5 4.5 0 0 0 2.5-4z" /></svg>
      <input
        v-model.number="volume"
        type="range"
        min="0"
        max="100"
        class="flex-1 accent-[var(--app-accent)]"
        @change="run(() => api.setVolume(volume))"
      />
    </div>
  </footer>
</template>
