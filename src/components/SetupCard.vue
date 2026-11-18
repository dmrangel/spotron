<script setup lang="ts">
import { ref } from "vue";
import { api, type Status } from "../player";

const props = defineProps<{ status: Status }>();
const emit = defineEmits<{ changed: [] }>();

const clientId = ref(props.status.client_id ?? "");
const busy = ref(false);
const error = ref("");

async function saveClientId() {
  error.value = "";
  try {
    await api.setClientId(clientId.value);
    emit("changed");
  } catch (e) {
    error.value = String(e);
  }
}

async function login() {
  error.value = "";
  busy.value = true;
  try {
    await api.login();
    emit("changed");
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="mx-auto mt-16 max-w-lg rounded-app-lg bg-app-card p-6 shadow-lg">
    <h1 class="mb-1 text-xl font-bold text-app-text-strong">Bem-vindo ao Spotron</h1>

    <template v-if="!status.client_id">
      <p class="mb-4 text-sm">
        Crie um app em <b class="text-app-text-strong">developer.spotify.com/dashboard</b> com o redirect URI
        <code class="rounded-app-sm bg-app-card-raised px-1 text-app-text-strong">{{ status.redirect_uri }}</code>,
        marque Web API e Web Playback SDK e cole o Client ID aqui.
      </p>
      <div class="flex gap-2">
        <input
          v-model="clientId"
          placeholder="Client ID (32 caracteres)"
          class="flex-1 rounded-app border border-app-border bg-app-panel px-3 py-2 text-app-text-strong outline-none focus:border-app-accent"
          @keydown.enter="saveClientId"
        />
        <button class="rounded-app bg-app-accent px-4 py-2 font-semibold text-app-bg" @click="saveClientId">Salvar</button>
      </div>
    </template>

    <template v-else>
      <p class="mb-4 text-sm">Entre com a sua conta Spotify Premium. O login abre no navegador.</p>
      <button
        class="w-full rounded-app bg-app-accent px-4 py-2 font-semibold text-app-bg disabled:opacity-50"
        :disabled="busy"
        @click="login"
      >
        {{ busy ? "Aguardando o navegador..." : "Entrar com o Spotify" }}
      </button>
    </template>

    <p v-if="error" class="mt-3 text-sm text-app-danger">{{ error }}</p>
  </div>
</template>
