<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { dsp, FILTER_LABELS, hasGain, type Band, type DspStatus, type FilterType, type Preset } from "../dsp";
import { player } from "../player";
import SpatialEffects from "./SpatialEffects.vue";

const enabled = ref(true);
const preset = ref<Preset | null>(null);
const presets = ref<string[]>([]);
const status = ref<DspStatus | null>(null);
const error = ref("");
const info = ref("");
const autoeqText = ref("");
const autoeqName = ref("");
const showAutoeq = ref(false);
const silentSince = ref<number | null>(null);

onMounted(async () => {
  const s = await dsp.get();
  enabled.value = s.enabled;
  preset.value = s.preset;
  presets.value = await dsp.presetList();
  poll();
});

// Estado do CamillaDSP a cada 500 ms (nível de sinal, clipping, erros).
let pollTimer = 0;
async function poll() {
  try {
    status.value = await dsp.status();
  } catch {
    status.value = null;
  }
  // Aviso de roteamento: tocando há > 4 s e nada chegando no cabo.
  const playing = player.state?.active && !player.state.paused;
  const silent = (status.value?.capture_rms ?? [-1000]).every((v) => v < -90);
  if (playing && silent) silentSince.value ??= Date.now();
  else silentSince.value = null;
  pollTimer = window.setTimeout(poll, 500);
}
onUnmounted(() => clearTimeout(pollTimer));

const routingProblem = computed(() => silentSince.value !== null && Date.now() - silentSince.value > 4000);
const level = (rms: number[] | null | undefined) => Math.max(-60, Math.max(...(rms ?? [-1000])));

// Aplica ao vivo com intervalo mínimo (o CamillaDSP limita a taxa de SetConfigJson).
let applyTimer = 0;
let applying = false;
let pending = false;
function apply() {
  clearTimeout(applyTimer);
  applyTimer = window.setTimeout(async () => {
    if (!preset.value) return;
    if (applying) {
      pending = true;
      return;
    }
    applying = true;
    try {
      await dsp.setPreset(JSON.parse(JSON.stringify(preset.value)));
      error.value = "";
    } catch (e) {
      error.value = String(e);
    } finally {
      applying = false;
      if (pending) {
        pending = false;
        apply();
      }
    }
  }, 60);
}

async function toggleEnabled() {
  enabled.value = !enabled.value;
  try {
    await dsp.setEnabled(enabled.value);
  } catch (e) {
    error.value = String(e);
  }
}

function addBand() {
  preset.value?.filters.push({ type: "peaking", freq: 1000, gain_db: 0, q: 1, enabled: true });
  apply();
}
function removeBand(i: number) {
  preset.value?.filters.splice(i, 1);
  apply();
}
function setType(b: Band, t: string) {
  b.type = t as FilterType;
  apply();
}

async function importAutoeq() {
  error.value = "";
  try {
    const name = autoeqName.value.trim() || "AutoEQ";
    preset.value!.correction = await dsp.parseAutoeq(name, autoeqText.value);
    showAutoeq.value = false;
    autoeqText.value = "";
    info.value = `Correção "${name}" importada (${preset.value!.correction.filters.length} filtros)`;
    apply();
  } catch (e) {
    error.value = String(e);
  }
}
async function loadAutoeqFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (!file) return;
  autoeqText.value = await file.text();
  autoeqName.value ||= file.name.replace(/ ParametricEQ\.txt$|\.txt$/i, "");
}

async function savePreset() {
  const name = window.prompt("Nome do preset:", preset.value?.name ?? "");
  if (!name || !preset.value) return;
  try {
    preset.value.name = name;
    await dsp.presetSave(preset.value, name);
    presets.value = await dsp.presetList();
    info.value = `Preset "${name}" salvo`;
    apply();
  } catch (e) {
    error.value = String(e);
  }
}
async function loadPreset(name: string) {
  if (!name) return;
  try {
    preset.value = await dsp.presetLoad(name);
    info.value = `Preset "${name}" carregado`;
    apply();
  } catch (e) {
    error.value = String(e);
  }
}
async function deletePreset() {
  const name = preset.value?.name;
  if (!name || !presets.value.includes(name) || !window.confirm(`Apagar o preset "${name}"?`)) return;
  await dsp.presetDelete(name);
  presets.value = await dsp.presetList();
}
function resetPreset() {
  if (!preset.value) return;
  preset.value = { ...preset.value, name: "Plano", preamp_db: 0, filters: [], correction: null };
  apply();
}
</script>

<template>
  <div v-if="preset" class="flex flex-col gap-4">
    <!-- Cabeçalho: liga/desliga, presets, estado -->
    <section class="flex flex-wrap items-center gap-3 rounded-app-lg bg-app-card p-4">
      <button
        class="rounded-app px-4 py-2 font-semibold"
        :class="enabled ? 'bg-app-accent text-app-bg' : 'bg-app-card-raised text-app-text'"
        @click="toggleEnabled"
      >
        {{ enabled ? "DSP ligado" : "DSP desligado (bypass)" }}
      </button>
      <select
        class="rounded-app border border-app-border bg-app-panel px-3 py-2 text-app-text-strong"
        :value="presets.includes(preset.name) ? preset.name : ''"
        @change="loadPreset(($event.target as HTMLSelectElement).value)"
      >
        <option value="" disabled>{{ preset.name }} (não salvo)</option>
        <option v-for="p in presets" :key="p" :value="p">{{ p }}</option>
      </select>
      <button class="rounded-app bg-app-card-raised px-3 py-2 text-app-text-strong" @click="savePreset">Salvar como…</button>
      <button class="rounded-app bg-app-card-raised px-3 py-2 text-app-text" @click="deletePreset">Apagar</button>
      <button class="rounded-app bg-app-card-raised px-3 py-2 text-app-text" @click="resetPreset">Zerar</button>

      <div class="ml-auto flex items-center gap-3 text-xs text-app-text-muted">
        <span>
          <span class="mr-1 inline-block h-2 w-2 rounded-full" :class="status?.running ? 'bg-app-accent' : 'bg-app-danger'" />
          CamillaDSP {{ status?.running ? status.state ?? "" : "parado" }}
        </span>
        <span>clip: {{ status?.clipped_samples ?? "-" }}</span>
        <div class="flex w-28 items-center gap-1" title="Nível de entrada (do cabo)">
          <span>in</span>
          <div class="h-2 flex-1 rounded-full bg-app-panel">
            <div class="h-2 rounded-full bg-app-accent" :style="{ width: `${((level(status?.capture_rms) + 60) / 60) * 100}%` }" />
          </div>
        </div>
      </div>
    </section>

    <section v-if="routingProblem" class="rounded-app-lg border border-app-danger bg-app-card p-4 text-sm">
      <b class="text-app-danger">O som do player não está chegando ao DSP.</b>
      No Mixer de volume do Windows, mude a saída do <b class="text-app-text-strong">Microsoft Edge</b> para
      <b class="text-app-text-strong">"CABLE Input (VB-Audio Virtual Cable)"</b>.
      <button class="ml-2 rounded-app bg-app-card-raised px-3 py-1 text-app-text-strong" @click="dsp.openVolumeMixer()">
        Abrir o Mixer de volume
      </button>
    </section>
    <p v-if="status?.error" class="text-sm text-app-danger">{{ status.error }}</p>

    <!-- Pré-amplificação -->
    <section class="rounded-app-lg bg-app-card p-4">
      <div class="flex items-center gap-4">
        <span class="w-40 font-semibold text-app-text-strong">Pré-amplificação</span>
        <input v-model.number="preset.preamp_db" type="range" min="-24" max="12" step="0.1" class="flex-1 accent-[var(--app-accent)]" @input="apply" />
        <span class="w-20 text-right tabular-nums">{{ preset.preamp_db.toFixed(1) }} dB</span>
      </div>
      <label class="mt-2 flex items-center gap-2 text-sm">
        <input v-model="preset.limiter.enabled" type="checkbox" class="accent-[var(--app-accent)]" @change="apply" />
        Limiter anti-clipping no fim da cadeia ({{ preset.limiter.clip_limit_db }} dBFS)
      </label>
    </section>

    <!-- Correção AutoEQ -->
    <section class="rounded-app-lg bg-app-card p-4">
      <div class="mb-2 flex items-center gap-3">
        <h2 class="font-semibold text-app-text-strong">Correção do fone (AutoEQ)</h2>
        <button class="ml-auto rounded-app bg-app-card-raised px-3 py-1 text-sm text-app-text-strong" @click="showAutoeq = !showAutoeq">
          Importar…
        </button>
      </div>
      <div v-if="preset.correction" class="flex items-center gap-3 text-sm">
        <label class="flex items-center gap-2">
          <input v-model="preset.correction.enabled" type="checkbox" class="accent-[var(--app-accent)]" @change="apply" />
          <b class="text-app-text-strong">{{ preset.correction.name }}</b>
        </label>
        <span class="text-app-text-muted">
          {{ preset.correction.filters.length }} filtros, preamp {{ preset.correction.preamp_db }} dB
        </span>
        <button class="ml-auto text-app-text-muted hover:text-app-danger" @click="preset.correction = null; apply()">Remover</button>
      </div>
      <p v-else class="text-sm text-app-text-muted">Nenhuma correção. Importe o "ParametricEQ.txt" do seu fone (autoeq.app).</p>
      <div v-if="showAutoeq" class="mt-3 flex flex-col gap-2">
        <div class="flex gap-2">
          <input v-model="autoeqName" placeholder="Nome (ex.: Sennheiser HD 650)" class="flex-1 rounded-app border border-app-border bg-app-panel px-3 py-1 text-app-text-strong" />
          <input type="file" accept=".txt" class="text-xs" @change="loadAutoeqFile" />
        </div>
        <textarea
          v-model="autoeqText"
          rows="5"
          placeholder="Preamp: -6.2 dB&#10;Filter 1: ON LSC Fc 105 Hz Gain 5.5 dB Q 0.71&#10;..."
          class="rounded-app border border-app-border bg-app-panel p-2 font-mono text-xs text-app-text-strong"
        />
        <button class="self-start rounded-app bg-app-accent px-4 py-1 font-semibold text-app-bg" @click="importAutoeq">Aplicar correção</button>
      </div>
    </section>

    <!-- EQ paramétrico -->
    <section class="rounded-app-lg bg-app-card p-4">
      <div class="mb-3 flex items-center">
        <h2 class="font-semibold text-app-text-strong">Equalizador paramétrico</h2>
        <button class="ml-auto rounded-app bg-app-card-raised px-3 py-1 text-sm text-app-text-strong" @click="addBand">+ Banda</button>
      </div>
      <p v-if="!preset.filters.length" class="text-sm text-app-text-muted">Sem bandas. Clique em "+ Banda".</p>
      <div v-for="(b, i) in preset.filters" :key="i" class="mb-2 flex items-center gap-3 rounded-app bg-app-panel px-3 py-2 text-sm">
        <input v-model="b.enabled" type="checkbox" class="accent-[var(--app-accent)]" @change="apply" />
        <select :value="b.type" class="rounded-app-sm border border-app-border bg-app-card px-2 py-1 text-app-text-strong" @change="setType(b, ($event.target as HTMLSelectElement).value)">
          <option v-for="(label, t) in FILTER_LABELS" :key="t" :value="t">{{ label }}</option>
        </select>
        <label class="flex items-center gap-1">
          <input v-model.number="b.freq" type="number" min="10" max="22000" class="w-20 rounded-app-sm border border-app-border bg-app-card px-2 py-1 text-right text-app-text-strong" @change="apply" />
          Hz
        </label>
        <input
          v-model.number="b.gain_db"
          type="range"
          min="-15"
          max="15"
          step="0.1"
          :disabled="!hasGain(b.type)"
          class="flex-1 accent-[var(--app-accent)] disabled:opacity-30"
          @input="apply"
        />
        <span class="w-16 text-right tabular-nums" :class="{ 'opacity-30': !hasGain(b.type) }">{{ b.gain_db.toFixed(1) }} dB</span>
        <label class="flex items-center gap-1">
          Q
          <input v-model.number="b.q" type="number" min="0.1" max="20" step="0.05" class="w-16 rounded-app-sm border border-app-border bg-app-card px-2 py-1 text-right text-app-text-strong" @change="apply" />
        </label>
        <button class="text-app-text-muted hover:text-app-danger" title="Remover banda" @click="removeBand(i)">✕</button>
      </div>
    </section>

    <SpatialEffects :preset="preset" @apply="apply" />

    <p v-if="info" class="text-sm text-app-accent">{{ info }}</p>
    <p v-if="error" class="text-sm text-app-danger">{{ error }}</p>
  </div>
</template>
