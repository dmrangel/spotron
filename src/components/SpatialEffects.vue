<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { CROSSFEED_PRESETS, dsp, type Preset, type RaceInfo } from "../dsp";

const props = defineProps<{ preset: Preset }>();
const emit = defineEmits<{ apply: [] }>();

type Mode = "none" | "crossfeed" | "race";
const mode = computed<Mode>(() =>
  props.preset.ambiophonics.enabled ? "race" : props.preset.crossfeed.enabled ? "crossfeed" : "none",
);

// Crossfeed (fones) e R.A.C.E. (caixas) se excluem, como no Neutron.
function setMode(m: Mode) {
  props.preset.crossfeed.enabled = m === "crossfeed";
  props.preset.ambiophonics.enabled = m === "race";
  emit("apply");
}

function useCrossfeedPreset(p: { cutoff_hz: number; level_db: number }) {
  props.preset.crossfeed.cutoff_hz = p.cutoff_hz;
  props.preset.crossfeed.level_db = p.level_db;
  emit("apply");
}

// O que o R.A.C.E. realmente aplica (atraso em amostras inteiras, compensação de ganho).
const race = ref<RaceInfo | null>(null);
watch(
  () => ({ ...props.preset.ambiophonics }),
  async (a) => {
    try {
      race.value = await dsp.raceInfo(a);
    } catch {
      race.value = null;
    }
  },
  { immediate: true },
);

const modes: { id: Mode; label: string; hint: string }[] = [
  { id: "none", label: "Nenhum", hint: "" },
  { id: "crossfeed", label: "Crossfeed", hint: "fones" },
  { id: "race", label: "Ambiophonic R.A.C.E.", hint: "caixas" },
];
</script>

<template>
  <section class="rounded-app-lg bg-app-card p-4">
    <h2 class="mb-3 font-semibold text-app-text-strong">Efeitos espaciais</h2>
    <div class="mb-4 flex gap-2">
      <button
        v-for="m in modes"
        :key="m.id"
        class="rounded-app px-3 py-1.5 text-sm font-semibold"
        :class="mode === m.id ? 'bg-app-accent-soft text-app-accent' : 'bg-app-card-raised text-app-text'"
        @click="setMode(m.id)"
      >
        {{ m.label }} <span v-if="m.hint" class="font-normal text-app-text-muted">({{ m.hint }})</span>
      </button>
    </div>

    <!-- Crossfeed -->
    <div v-if="mode === 'crossfeed'" class="flex flex-col gap-3 text-sm">
      <p class="text-app-text-muted">
        Mistura um pouco de cada canal no outro, filtrado e atrasado, para o som não ficar "dentro da cabeça" nos fones.
      </p>
      <div class="flex gap-2">
        <button
          v-for="p in CROSSFEED_PRESETS"
          :key="p.label"
          class="rounded-app-sm bg-app-card-raised px-3 py-1 text-app-text-strong"
          @click="useCrossfeedPreset(p)"
        >
          {{ p.label }}
        </button>
      </div>
      <label class="flex items-center gap-3">
        <span class="w-32">Corte</span>
        <input v-model.number="preset.crossfeed.cutoff_hz" type="range" min="300" max="2000" step="10" class="flex-1 accent-[var(--app-accent)]" @input="emit('apply')" />
        <span class="w-20 text-right tabular-nums">{{ preset.crossfeed.cutoff_hz }} Hz</span>
      </label>
      <label class="flex items-center gap-3">
        <span class="w-32">Nível</span>
        <input v-model.number="preset.crossfeed.level_db" type="range" min="1" max="15" step="0.5" class="flex-1 accent-[var(--app-accent)]" @input="emit('apply')" />
        <span class="w-20 text-right tabular-nums">{{ preset.crossfeed.level_db.toFixed(1) }} dB</span>
      </label>
    </div>

    <!-- R.A.C.E. -->
    <div v-if="mode === 'race'" class="flex flex-col gap-3 text-sm">
      <p class="text-app-text-muted">
        Cancelamento de crosstalk para <b>caixas de som</b> próximas, com o ouvinte centralizado (não serve para fones).
        Calibração: toque algo só no canal esquerdo e ajuste o menor atraso e a maior atenuação que deixam o som
        totalmente à esquerda.
      </p>
      <label class="flex items-center gap-3">
        <span class="w-32">Atraso</span>
        <input v-model.number="preset.ambiophonics.delay_us" type="range" min="40" max="140" step="10" class="flex-1 accent-[var(--app-accent)]" @input="emit('apply')" />
        <span class="w-36 text-right tabular-nums">
          {{ preset.ambiophonics.delay_us }} µs
          <span class="text-app-text-muted">→ {{ race?.delay_us.toFixed(1) ?? "?" }}</span>
        </span>
      </label>
      <label class="flex items-center gap-3">
        <span class="w-32">Atenuação</span>
        <input v-model.number="preset.ambiophonics.attenuation_db" type="range" min="1" max="8" step="0.5" class="flex-1 accent-[var(--app-accent)]" @input="emit('apply')" />
        <span class="w-36 text-right tabular-nums">{{ preset.ambiophonics.attenuation_db.toFixed(1) }} dB</span>
      </label>
      <label class="flex items-center gap-3">
        <span class="w-32">Faixa: de</span>
        <input v-model.number="preset.ambiophonics.low_hz" type="range" min="100" max="600" step="10" class="flex-1 accent-[var(--app-accent)]" @input="emit('apply')" />
        <span class="w-36 text-right tabular-nums">{{ preset.ambiophonics.low_hz }} Hz</span>
      </label>
      <label class="flex items-center gap-3">
        <span class="w-32">até</span>
        <input v-model.number="preset.ambiophonics.high_hz" type="range" min="2000" max="12000" step="100" class="flex-1 accent-[var(--app-accent)]" @input="emit('apply')" />
        <span class="w-36 text-right tabular-nums">{{ preset.ambiophonics.high_hz }} Hz</span>
      </label>
      <p v-if="race" class="text-xs text-app-text-muted">
        Aplicado: atraso de {{ race.delay_samples }} amostras ({{ race.delay_us.toFixed(1) }} µs a 48 kHz),
        compensação de ganho {{ race.gain_db.toFixed(1) }} dB, FIR de {{ race.taps }} coeficientes.
      </p>
    </div>
  </section>
</template>
