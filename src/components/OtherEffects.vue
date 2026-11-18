<script setup lang="ts">
import type { Preset } from "../dsp";

defineProps<{ preset: Preset }>();
const emit = defineEmits<{ apply: [] }>();
const apply = () => emit("apply");

function balanceLabel(b: number) {
  if (Math.abs(b) < 0.005) return "centro";
  return `${Math.round(Math.abs(b) * 100)}% ${b < 0 ? "esq." : "dir."}`;
}
</script>

<template>
  <!-- Loudness -->
  <section class="rounded-app-lg bg-app-card p-4 text-sm">
    <label class="mb-2 flex items-center gap-2">
      <input v-model="preset.loudness.enabled" type="checkbox" class="accent-[var(--app-accent)]" @change="apply" />
      <h2 class="font-semibold text-app-text-strong">Loudness adaptativo</h2>
    </label>
    <p class="mb-3 text-app-text-muted">
      Em volume baixo o ouvido perde graves e agudos; o loudness compensa conforme o volume do app (abaixo do nível de
      referência).
    </p>
    <div v-if="preset.loudness.enabled" class="flex flex-col gap-2">
      <label class="flex items-center gap-3">
        <span class="w-40">Referência</span>
        <input v-model.number="preset.loudness.reference_level_db" type="range" min="-50" max="0" step="1" class="flex-1 accent-[var(--app-accent)]" @input="apply" />
        <span class="w-20 text-right tabular-nums">{{ preset.loudness.reference_level_db }} dB</span>
      </label>
      <label class="flex items-center gap-3">
        <span class="w-40">Reforço de graves</span>
        <input v-model.number="preset.loudness.low_boost_db" type="range" min="0" max="20" step="0.5" class="flex-1 accent-[var(--app-accent)]" @input="apply" />
        <span class="w-20 text-right tabular-nums">{{ preset.loudness.low_boost_db.toFixed(1) }} dB</span>
      </label>
      <label class="flex items-center gap-3">
        <span class="w-40">Reforço de agudos</span>
        <input v-model.number="preset.loudness.high_boost_db" type="range" min="0" max="20" step="0.5" class="flex-1 accent-[var(--app-accent)]" @input="apply" />
        <span class="w-20 text-right tabular-nums">{{ preset.loudness.high_boost_db.toFixed(1) }} dB</span>
      </label>
    </div>
  </section>

  <!-- Canais -->
  <section class="rounded-app-lg bg-app-card p-4 text-sm">
    <h2 class="mb-3 font-semibold text-app-text-strong">Canais</h2>
    <label class="mb-3 flex items-center gap-3">
      <span class="w-40">Balanço</span>
      <input v-model.number="preset.channels.balance" type="range" min="-1" max="1" step="0.01" class="flex-1 accent-[var(--app-accent)]" @input="apply" @dblclick="preset.channels.balance = 0; apply()" />
      <span class="w-20 text-right tabular-nums">{{ balanceLabel(preset.channels.balance) }}</span>
    </label>
    <div class="mb-3 flex flex-wrap gap-x-6 gap-y-2">
      <label class="flex items-center gap-2"><input v-model="preset.channels.mono" type="checkbox" class="accent-[var(--app-accent)]" @change="apply" /> Mono</label>
      <label class="flex items-center gap-2" :class="{ 'opacity-40': preset.channels.mono }">
        <input v-model="preset.channels.swap" type="checkbox" class="accent-[var(--app-accent)]" :disabled="preset.channels.mono" @change="apply" /> Trocar esquerda/direita
      </label>
      <label class="flex items-center gap-2"><input v-model="preset.channels.invert_left" type="checkbox" class="accent-[var(--app-accent)]" @change="apply" /> Inverter fase (esq.)</label>
      <label class="flex items-center gap-2"><input v-model="preset.channels.invert_right" type="checkbox" class="accent-[var(--app-accent)]" @change="apply" /> Inverter fase (dir.)</label>
    </div>
    <p class="mb-2 text-app-text-muted">Alinhamento de tempo: atrase a caixa mais próxima (1 ms ≈ 34 cm).</p>
    <div class="flex gap-6">
      <label class="flex items-center gap-2">
        Esquerda
        <input v-model.number="preset.channels.delay_left_ms" type="number" min="0" max="50" step="0.05" class="w-20 rounded-app-sm border border-app-border bg-app-panel px-2 py-1 text-right text-app-text-strong" @change="apply" />
        ms
      </label>
      <label class="flex items-center gap-2">
        Direita
        <input v-model.number="preset.channels.delay_right_ms" type="number" min="0" max="50" step="0.05" class="w-20 rounded-app-sm border border-app-border bg-app-panel px-2 py-1 text-right text-app-text-strong" @change="apply" />
        ms
      </label>
    </div>
  </section>

  <!-- Proteção -->
  <section class="rounded-app-lg bg-app-card p-4 text-sm">
    <label class="mb-2 flex items-center gap-2">
      <input v-model="preset.protection.enabled" type="checkbox" class="accent-[var(--app-accent)]" @change="apply" />
      <h2 class="font-semibold text-app-text-strong">Proteção de caixas</h2>
    </label>
    <p class="mb-3 text-app-text-muted">Corta frequências que as caixas não reproduzem e só gastam potência (Butterworth 24 dB/oitava). 0 = desligado.</p>
    <div v-if="preset.protection.enabled" class="flex gap-6">
      <label class="flex items-center gap-2">
        Subsônico
        <input v-model.number="preset.protection.subsonic_hz" type="number" min="0" max="200" step="1" class="w-20 rounded-app-sm border border-app-border bg-app-panel px-2 py-1 text-right text-app-text-strong" @change="apply" />
        Hz
      </label>
      <label class="flex items-center gap-2">
        Ultrassônico
        <input v-model.number="preset.protection.ultrasonic_hz" type="number" min="0" max="23000" step="100" class="w-24 rounded-app-sm border border-app-border bg-app-panel px-2 py-1 text-right text-app-text-strong" @change="apply" />
        Hz
      </label>
    </div>
  </section>
</template>
