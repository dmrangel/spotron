<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import EqPage from "./components/EqPage.vue";
import PlayerBar from "./components/PlayerBar.vue";
import SetupCard from "./components/SetupCard.vue";
import { api, player, startListening, type Status } from "./player";

const status = ref<Status | null>(null);
const uri = ref("");
const message = ref("");
const error = ref("");
const showLog = ref(false);
const page = ref<"play" | "eq">("play");
const pages = [
  { id: "play", label: "Tocar" },
  { id: "eq", label: "Equalizador" },
] as const;

async function refresh() {
  status.value = await api.status();
}

onMounted(async () => {
  startListening();
  await refresh();
});

const ready = computed(() => !!player.state?.ready || !!status.value?.device_ready);
const loggedIn = computed(() => !!status.value?.client_id && !!status.value?.logged_in);

function showError(msg: string) {
  error.value = msg;
  message.value = "";
}

async function playUri() {
  error.value = "";
  try {
    const played = await api.playUri(uri.value);
    message.value = `Tocando ${played}`;
    uri.value = "";
  } catch (e) {
    showError(String(e));
  }
}

async function transfer() {
  error.value = "";
  try {
    await api.transferHere();
    message.value = "Reprodução trazida para o Spotron";
  } catch (e) {
    showError(String(e));
  }
}

async function logout() {
  await api.logout();
  player.state = null;
  await refresh();
}
</script>

<template>
  <div v-if="status" class="flex h-full flex-col">
    <SetupCard v-if="!loggedIn" :status="status" @changed="refresh" />

    <template v-else>
      <div class="flex min-h-0 flex-1">
        <!-- Barra lateral -->
        <aside class="flex w-56 flex-col gap-1 border-r border-app-border bg-app-panel p-3">
          <div class="mb-4 px-2 text-lg font-bold text-app-text-strong">Spotron</div>
          <button
            v-for="p in pages"
            :key="p.id"
            class="rounded-app px-3 py-2 text-left font-semibold"
            :class="page === p.id ? 'bg-app-accent-soft text-app-accent' : 'text-app-text hover:bg-app-card'"
            @click="page = p.id"
          >
            {{ p.label }}
          </button>
          <div class="mt-auto flex items-center gap-2 px-2 text-xs text-app-text-muted">
            <span class="h-2 w-2 rounded-full" :class="ready ? 'bg-app-accent' : 'bg-app-danger'" />
            {{ ready ? "Player conectado" : "Conectando o player..." }}
          </div>
          <button class="px-2 text-left text-xs text-app-text-muted hover:text-app-text" @click="logout">Sair da conta</button>
        </aside>

        <!-- Conteúdo -->
        <main class="flex-1 overflow-y-auto p-6">
          <EqPage v-if="page === 'eq'" />
          <template v-else>
          <section class="mb-4 rounded-app-lg bg-app-card p-5">
            <h2 class="mb-3 font-semibold text-app-text-strong">Tocar um link do Spotify</h2>
            <div class="flex gap-2">
              <input
                v-model="uri"
                placeholder="https://open.spotify.com/track/...  ou  spotify:album:..."
                class="flex-1 rounded-app border border-app-border bg-app-panel px-3 py-2 text-app-text-strong outline-none focus:border-app-accent"
                @keydown.enter="playUri"
              />
              <button
                class="rounded-app bg-app-accent px-4 py-2 font-semibold text-app-bg disabled:opacity-50"
                :disabled="!ready || !uri.trim()"
                @click="playUri"
              >
                Tocar
              </button>
            </div>
            <p class="mt-2 text-xs text-app-text-muted">Faixas, álbuns, playlists, artistas e podcasts.</p>
          </section>

          <section class="mb-4 rounded-app-lg bg-app-card p-5">
            <h2 class="mb-1 font-semibold text-app-text-strong">Dispositivo "Spotron"</h2>
            <p class="mb-3 text-sm">
              O Spotron também aparece no celular e no app oficial como dispositivo. Para trazer o que está tocando em
              outro lugar:
            </p>
            <button
              class="rounded-app bg-app-card-raised px-4 py-2 font-semibold text-app-text-strong disabled:opacity-50"
              :disabled="!ready"
              @click="transfer"
            >
              Trazer a reprodução para cá
            </button>
          </section>

          <p v-if="message" class="text-sm text-app-accent">{{ message }}</p>
          <p v-if="error" class="text-sm text-app-danger">{{ error }}</p>
          <p v-if="player.state?.error" class="text-sm text-app-danger">Player: {{ player.state.error }}</p>

          <button class="mt-6 text-xs text-app-text-muted hover:text-app-text" @click="showLog = !showLog">
            {{ showLog ? "Esconder" : "Mostrar" }} log do player
          </button>
          <pre
            v-if="showLog"
            class="mt-2 max-h-48 overflow-y-auto rounded-app bg-app-panel p-3 text-xs text-app-text-muted"
          >{{ player.logs.join("\n") }}</pre>
          </template>
        </main>
      </div>

      <PlayerBar @error="showError" />
    </template>
  </div>
</template>
