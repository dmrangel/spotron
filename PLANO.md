# Player Spotify customizado (Windows)

Player desktop leve para Windows que toca músicas do Spotify (conta Premium) com DSP próprio (EQ paramétrico, compressor, crossfeed, presets AutoEQ) e interface totalmente customizável, sem depender do app oficial do Spotify.

> **Aviso:** o app usa o **Spotify Web Playback SDK** e a **Web API**, que são oficiais, num app de desenvolvedor em modo de desenvolvimento (uso pessoal). O áudio do SDK sai pelo Windows e é capturado de um cabo virtual para aplicar o DSP em tempo real. As regras de desenvolvedor do Spotify proíbem gravar, armazenar ou fazer "ripping" do conteúdo (o app **nunca** grava nada) e trazem uma cláusula genérica contra "modificar/alterar" o conteúdo. EQ em tempo real não é citado diretamente: é uma zona cinzenta. Uso pessoal apenas, sem distribuição, sem uso comercial e sem nenhuma função de download ou gravação.

---

## Objetivos

1. **Estabilidade e leveza:** substituir o app oficial (que trava nesta máquina) por um app próprio. Meta de memória: < 250 MB com a UI aberta e tocando (inclui o processo Widevine do WebView2, ~55 MB).
2. **Som:** EQ paramétrico multibanda, pré-amplificação, compressor/limitador, crossfeed para fones, importação de presets AutoEQ (no estilo do Neutron).
3. **Visual:** temas customizáveis (CSS), visualizador de áudio em tempo real.
4. **Portabilidade futura:** o formato de presets de DSP deve ser independente da plataforma, para ser reaproveitado num futuro projeto Android separado.

## Fora do escopo

- Android (projeto separado no futuro).
- Lossless: o Web Playback SDK entrega **AAC 256 kbps** (Premium); o FLAC do Spotify só existe nos apps oficiais.
- Download, cache offline ou gravação de músicas.
- Distribuição pública ou uso comercial.

---

## Arquitetura

```
┌──────────────────────────── App Tauri 2 ────────────────────────────┐
│                                                                      │
│  Frontend (WebView2 + Widevine)          Backend Rust                │
│  ┌────────────────────────────┐         ┌──────────────────────────┐ │
│  │ UI + temas CSS             │◄─events─┤ OAuth PKCE + tokens      │ │
│  │ Editor de EQ, Visualizador │─invoke─►│ Cliente Web API          │ │
│  │ Spotify Web Playback SDK   │         │ Processo CamillaDSP      │ │
│  └─────────────┬──────────────┘         │ Cliente WebSocket (DSP)  │ │
│                │ áudio (DRM, AAC 256)   │ Captura p/ FFT ──────────┼─► visualizador
│                ▼                        └──────────────────────────┘ │
└────────────────┼─────────────────────────────────────────────────────┘
                 ▼  saída do app → "CABLE Input (VB-Audio Virtual Cable)"
        ┌───────────────────────────────────┐
        │ camilladsp.exe (sidecar)          │◄── WebSocket (controle do DSP)
        │ capture: Wasapi "CABLE Output"    │
        │ rate adjust + AsyncSinc           │
        │ playback: Wasapi (fones/padrão)   │
        └───────────────────────────────────┘
```

O áudio do SDK é protegido por DRM e é tocado pelo próprio WebView2: o código do app não tem acesso ao PCM. O único ponto de inserção do DSP é depois da saída do Windows. Por isso o app toca num **cabo de áudio virtual** e o CamillaDSP captura dele.

### Decisões e motivos

| Decisão | Motivo | Alternativa descartada |
|---|---|---|
| **Tauri 2** para a UI | WebView2 do sistema: binário pequeno, pouca RAM, temas em CSS. O WebView2 do Windows 11 já traz o Widevine (relatos de 2026; não documentado oficialmente pela Microsoft). | Electron com castLabs ECS (exige assinatura VMP via EVS a cada build; fica como último recurso). Chrome/Edge em modo `--app` (fica como plano B). |
| **Spotify Web Playback SDK** para tocar | Oficial, toca com a conta do usuário (o open.spotify.com toca nesta máquina). Aparece como dispositivo Connect, controlável também pelo celular. | **librespot**: o Spotify recusa a chave de áudio desta conta (`audio key error 0 1`, librespot#1649, sem workaround). Testado em 17/11/2026. |
| **Web API** (OAuth PKCE) para busca, biblioteca e fila | Oficial. No modo de desenvolvimento de 2026 ainda tem player, fila, busca, playlists próprias e biblioteca. | Scraping ou APIs internas (frágil e pior nos termos). |
| **VB-CABLE** como ponte de áudio | Gratuito, assinado, usado no exemplo da doc do CamillaDSP. O CamillaDSP 4.1 não faz captura por processo. | Loopback do dispositivo inteiro (pega o som de todos os apps). Virtual Audio Cable (pago). Drivers open source (exigem test-signing). |
| **CamillaDSP como sidecar** | Motor de DSP maduro: biquads, compressor, limiter, delay, convolução FIR, API WebSocket. Saída WASAPI já provada nesta máquina. | DSP próprio em Rust (fica para o projeto Android). |
| **Presets em JSON próprio** | Formato portátil. O app converte JSON → config do CamillaDSP. O futuro app Android lê o mesmo JSON. | Salvar YAML do CamillaDSP direto (amarra o formato ao motor). |

---

## Estrutura do repositório (proposta)

```
/
├─ PLANO.md                  ← este arquivo
├─ CLAUDE.md                 ← instruções para sessões do Claude Code
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ spotify/            ← OAuth PKCE, tokens, cliente da Web API
│  │  ├─ dsp/                ← processo CamillaDSP, cliente WebSocket, conversor de presets
│  │  ├─ audio/              ← roteamento para o VB-CABLE, detecção de dispositivos
│  │  ├─ viz/                ← captura do cabo + FFT → eventos para o frontend
│  │  └─ commands.rs         ← comandos Tauri expostos à UI
│  ├─ binaries/              ← camilladsp-x86_64-pc-windows-msvc.exe (sidecar)
│  └─ tauri.conf.json
├─ src/                      ← frontend Vue 3 + Tailwind (mesma stack do Modrinth App)
│  ├─ player/                ← wrapper do Web Playback SDK
│  ├─ themes/                ← temas CSS (modrinth-dark.css é o padrão)
│  ├─ tokens.css             ← camada de tokens semânticos do app
│  └─ components/
├─ presets/                  ← presets de EQ em JSON (formato próprio)
└─ spikes/                   ← scripts e configs da Fase 0 (spikes/arquivo/ = tentativas descartadas)
```

---

## Design system: tokens do Modrinth

A interface segue o visual do **Modrinth App** (launcher de Minecraft). O Modrinth App é feito com **Tauri + Vue + Tailwind**, a mesma stack deste projeto, então os tokens encaixam sem adaptação.

### Fonte oficial

- Repositório: `github.com/modrinth/code` (monorepo).
- Tokens: `packages/assets/styles/variables.scss` (variáveis CSS por tema: claro, escuro, OLED).
- Preset Tailwind: `packages/assets/tailwind-preset.ts`.
- Componentes Vue: `packages/ui/`.

### Licença (verificar antes de copiar qualquer arquivo)

O monorepo usa licenças diferentes por pacote: ler o `LICENSE` de `packages/assets` e de `packages/ui`.
- **Valores de cor e espaçamento** podem ser reproduzidos à mão sem problema.
- **Copiar arquivos ou componentes inteiros** traz a licença junto. Se for GPL/AGPL, este repositório precisa ser compatível. Para uso pessoal sem distribuição isso não pesa, mas fica registrado.
- **Não usar** logo, nome ou ícones da marca Modrinth.

### Arquitetura de tokens (duas camadas)

O app nunca usa os nomes do Modrinth direto nos componentes. Os componentes usam tokens semânticos **do app**, e cada tema mapeia esses tokens para valores. Assim o tema Modrinth é o padrão, mas o objetivo de "temas customizáveis" continua de pé.

```css
/* tokens.css: o que os componentes usam */
:root {
  --app-bg:          var(--surface-1);
  --app-panel:       var(--surface-2);
  --app-card:        var(--surface-3);
  --app-card-raised: var(--surface-4);
  --app-border:      var(--surface-5);
  --app-text-strong: var(--color-text-primary);
  --app-text:        var(--color-text-default);
  --app-text-muted:  var(--color-text-tertiary);
  --app-accent:      var(--color-brand);
}
```

```css
/* themes/modrinth-dark.css: valores copiados de variables.scss */
.theme-modrinth-dark {
  --surface-1: /* copiar de .dark-mode */;
  --surface-2: /* ... */;
  --surface-3: /* ... */;
  --surface-4: /* ... */;
  --surface-5: /* ... */;
  --color-text-primary: #ffffff;   /* conferido em out/2025 */
  --color-text-default: #b0bac5;   /* conferido em out/2025 */
  --color-text-tertiary: /* conferir: houve PR alterando de #80878f para #96a2b0 e depois revertendo */;
  --color-brand: /* copiar do arquivo */;
  /* raio de borda, sombras e fontes: copiar do arquivo também */
}
```

Valores de referência já conferidos para o tema **OLED** (out/2025, podem ter mudado): `--surface-1: #000000`, `--surface-2: #101013`, `--surface-3: #1b1b20`, `--surface-4: #25262b`, `--surface-5: #2e2f34`.

### Uso dos níveis de superfície (convenção do Modrinth)

| Token | Uso |
|---|---|
| surface-1 | Fundo da janela |
| surface-2 | Barra lateral, painéis |
| surface-3 | Cards |
| surface-4 | Cards elevados, hover |
| surface-5 | Bordas, divisores |

Regra herdada do Modrinth: **nunca usar cores fixas** (`#fff`, `bg-gray-800`) nos componentes, sempre os tokens.

### Temas iniciais

1. `modrinth-dark` (padrão)
2. `modrinth-oled`
3. `modrinth-light`

Tudo além disso (temas do usuário) entra na Fase 4.

---

## Fases

Cada fase só começa quando a anterior cumpre os critérios de aceite.

### Fase 0: Spikes da cadeia (sem app de verdade, sem UI)

**Objetivo:** provar, nesta máquina Windows, as três peças que podem inviabilizar o projeto. Arquivos em `spikes/fase0/`.

**Pré-requisitos (feitos pelo usuário):**
- Criar um app em `developer.spotify.com/dashboard` com a conta Premium. Redirect URI `http://127.0.0.1:8888/callback` (`localhost` não é aceito desde 2025). Marcar Web API e Web Playback SDK. Adicionar a própria conta em User Management.
- Instalar o **VB-CABLE** (vb-audio.com/Cable, pede administrador e reinício). Pôr "CABLE Input" e "CABLE Output" em 48000 Hz / 2 canais no Painel de Som. Manter os fones como saída padrão.

#### Spike A: Web Playback SDK dentro do WebView2 (Tauri mínimo)

1. App Tauri 2 mínimo com uma página que carrega `https://sdk.scdn.co/spotify-player.js` e cria o `Spotify.Player` com um token de escopo `streaming` (no spike, colado à mão ou obtido pelo Spike C).
2. Chamar `player.activateElement()` num clique (política de autoplay).
3. Logar `navigator.requestMediaKeySystemAccess('com.widevine.alpha', ...)` e os eventos de erro do SDK (`initialization_error`, `authentication_error`, `account_error`, `playback_error`).
4. Transferir a reprodução para o dispositivo pelo celular ou pelo `PUT /me/player`.

**Critérios de aceite:**
- [ ] Widevine disponível no WebView2 (`requestMediaKeySystemAccess` resolve).
- [ ] Toca 10 minutos seguidos, com troca de faixa e seek, sem erro de licença.
- [ ] Plano B só se falhar: Chrome/Edge em `--app=http://127.0.0.1:<porta>` com `--user-data-dir` próprio.

#### Spike B: VB-CABLE → CamillaDSP → fones

1. Rotear a saída do app para "CABLE Input" (Configurações > Sistema > Som > Mixer de volume > app > Dispositivo de saída). **Verificar** qual processo aparece no mixer: o áudio do WebView2 sai de um `msedgewebview2.exe`, que pode ser compartilhado com outros apps (Teams, Widgets). Se for, testar o WebView2 *Fixed Version* empacotado com o app (caminho de executável próprio).
2. Rodar `camilladsp.exe -p 1234 -v vbcable.yml` (captura "CABLE Output", playback no dispositivo padrão, `enable_rate_adjust` + `AsyncSinc`).
3. Repetir o `ws_test.py` (passa-baixa 500 Hz ↔ 20 kHz, varredura do peaking).

**Critérios de aceite:**
- [ ] 10 min sem estalos (anotar `chunksize` e `target_level`).
- [ ] Rate adjust estável: sem underrun/overrun no `camilladsp.log` (anotar a taxa de captura medida).
- [ ] Passa-baixa de 500 Hz audível.
- [ ] Mudança via WebSocket sem interromper a música (anotar a latência impressa).
- [ ] Só o áudio do app passa pelo cabo; outros apps tocam normalmente nos fones.
- [ ] Formato documentado: AAC 256 → WebView2 → cabo a 48000 Hz F32 → CamillaDSP 48000 Hz F32 → fones.

#### Spike C: OAuth PKCE + Web API

1. Fluxo Authorization Code com PKCE, redirect em `http://127.0.0.1:8888/callback`, escopos `streaming user-read-email user-read-private user-read-playback-state user-modify-playback-state playlist-read-private playlist-read-collaborative user-library-read`.
2. Refresh token salvo e renovado.
3. Chamar: `GET /search` (máx. 10 por página), `GET /me/playlists`, `GET /playlists/{id}/items`, `GET /me/tracks`, `POST /me/player/queue`, `PUT /me/player` (transferir para o dispositivo do SDK).

**Critérios de aceite:**
- [ ] Login funciona e o refresh token renova sem novo login.
- [ ] Busca, playlists próprias, curtidas e fila respondem.
- [ ] Anotar o que falha (ex.: itens de playlists seguidas mas não próprias) e o que isso tira da Fase 3.

**RAM ao fim da Fase 0:** anotar processos do WebView2 (incluindo o utilitário Widevine) e `camilladsp.exe`.

### Fase 1: Esqueleto Tauri + player

1. Criar o app Tauri 2 de verdade (Vue 3 + Tailwind).
2. Login OAuth PKCE no backend Rust, com tokens em `%APPDATA%` (refresh automático).
3. Wrapper do Web Playback SDK no frontend; o backend entrega o token via comando Tauri.
4. Expor ao frontend `play(uri)`, `pause`, `resume`, `next`, `prev`, `seek`, `volume`, via SDK e Web API.
5. O app aparece como dispositivo Spotify Connect, controlável também pelo celular.

**Critérios de aceite:**
- [ ] Login funciona e persiste entre reinícios.
- [ ] Tocar uma URI colada na UI funciona.
- [ ] Controles básicos funcionam.
- [ ] RAM do app medida e anotada.

### Fase 2: DSP integrado

1. Tauri inicia o `camilladsp.exe` como sidecar ao abrir e encerra ao fechar.
2. O app garante que a própria saída vá para o "CABLE Input" (automatizar a preferência por app, se possível, ou instruir o usuário na primeira execução) e avisa se o VB-CABLE não estiver instalado.
3. Cliente WebSocket no backend para ler e alterar a config.
4. Definir o **formato de preset JSON**, por exemplo:
   ```json
   {
     "name": "HD650 AutoEQ",
     "preamp_db": -6.2,
     "filters": [
       { "type": "peaking", "freq": 105, "gain_db": -3.1, "q": 0.7 },
       { "type": "lowshelf", "freq": 105, "gain_db": 5.5, "q": 0.71 }
     ],
     "compressor": null,
     "crossfeed": { "enabled": false, "level": "medium" }
   }
   ```
5. Conversor preset JSON → config do CamillaDSP.
6. Importador do formato paramétrico do AutoEQ (`Filter N: ON PK Fc X Hz Gain Y dB Q Z`).
7. Proteção contra clipping: limiter no fim da cadeia, ligado por padrão.
8. Reiniciar o CamillaDSP quando o dispositivo de saída mudar (fone conectado/desconectado).

**Critérios de aceite:**
- [ ] Mudar uma banda na UI altera o som em < 100 ms, sem cortes.
- [ ] Importar um preset AutoEQ real funciona.
- [ ] Fechar o app não deixa o `camilladsp.exe` órfão.
- [ ] Trocar o fone não deixa o app mudo.

### Fase 3: Interface e navegação

1. Busca, biblioteca, playlists e fila via Web API, respeitando os limites do modo de desenvolvimento de 2026 (busca com no máximo 10 por página; itens só de playlists próprias ou colaborativas; sem recomendações, rádio ou artistas relacionados).
2. Editor visual de EQ (curva de resposta desenhada a partir dos biquads, com pontos arrastáveis).
3. Sistema de temas: implementar a arquitetura de duas camadas da seção "Design system", começando pelos três temas Modrinth, com troca em tempo real.
4. Layout inspirado no Modrinth App: barra lateral de navegação fixa (surface-2), conteúdo em cards (surface-3) e barra de reprodução fixa embaixo.

### Fase 4: Visualizador e polimento

1. Capturar o áudio do "CABLE Output" em paralelo ao CamillaDSP (WASAPI shared aceita vários clientes de captura), calcular a FFT (crate `rustfft`) e emitir eventos Tauri a ~30–60 fps. Só análise em memória, nada é gravado.
2. Visualizador no frontend (canvas/WebGL).
3. Integração com as mídias do Windows (SMTC: teclas de mídia e overlay de volume; o SDK tem `enableMediaSession`).
4. Bandeja do sistema, atalhos globais e inicialização com o Windows (opcional).

---

## Riscos conhecidos

| Risco | Impacto | Mitigação |
|---|---|---|
| Microsoft muda o suporte a Widevine no WebView2 (não documentado) | SDK para de tocar | Plano B: Chrome/Edge em modo `--app`. Último recurso: Electron castLabs ECS com assinatura EVS. |
| Spotify restringe ainda mais o modo de desenvolvimento ou o SDK | Perda de busca/biblioteca ou do player | Acompanhar o blog de desenvolvedores; o app oficial fica como reserva. |
| Termos de desenvolvedor: "modificar/alterar" e "capturar" o stream | App de desenvolvedor suspenso | Uso pessoal, nada gravado, nada distribuído. Risco baixo, mas não zero. |
| O roteamento por app pega o `msedgewebview2.exe` compartilhado | Áudio de outros apps WebView2 passa pelo DSP, ou o roteamento se perde | Testar no Spike B; WebView2 Fixed Version empacotado; ou `setSinkId` injetado no iframe do SDK. |
| Clock do VB-CABLE diferente do da placa | Estalos periódicos ou buffer crescendo | `enable_rate_adjust` + `AsyncSinc` no CamillaDSP; medir no Spike B. |
| Mudança de dispositivo de áudio no Windows (fone conectado/desconectado) | CamillaDSP perde a saída | Detectar o erro e reiniciar o sidecar com o dispositivo novo. |
| Qualidade limitada a AAC 256 | Teto de qualidade abaixo do app oficial | Aceito: o objetivo é DSP e estabilidade, não lossless. |

---

## Registro de decisões

| Data | Decisão | Motivo |
|---|---|---|
| 2026-11-17 | CamillaDSP fixado em 4.1.3 (5.0 ainda beta). | Última estável. |
| 2026-11-17 | ~~librespot 0.8.0 com `--backend pipe`.~~ **Descartado.** | Testado nesta máquina: o Spotify recusa a chave de áudio da conta em todas as faixas (`error audio key 0 1` → `Unable to load key` → `Symphonia ... end of stream`). Restrição do lado do Spotify (librespot#1649), afeta todas as versões, sem workaround; OAuth também falha. Registro em `spikes/arquivo/librespot/`. |
| 2026-11-17 | Saída WASAPI do CamillaDSP validada: "Fones de ouvido (Realtek(R) Audio)", shared, 48000 Hz F32. | Medido no spike do librespot (o lado do CamillaDSP funcionou). Os dois dispositivos Realtek estão em 48000 Hz. |
| 2026-11-17 | Fonte do áudio: Spotify Web Playback SDK (oficial) + Web API, app de desenvolvedor em modo de desenvolvimento. | Única forma de tocar Spotify sem o app oficial com esta conta. O open.spotify.com toca no navegador do usuário. Limites aceitos: AAC 256, até 5 usuários, busca com 10 por página. |
| 2026-11-17 | DSP via VB-CABLE → CamillaDSP (captura WASAPI + rate adjust AsyncSinc). | O áudio do SDK é DRM: o app não tem o PCM. O CamillaDSP 4.1 não faz captura por processo. |
| 2026-11-17 | Mantido o Tauri 2 (WebView2 com Widevine). | Relatos de 2026 de SDK/open.spotify.com tocando no WebView2 sem configuração. A confirmar no Spike A. |
| 2026-11-17 | Deezer (pleezer) considerado e deixado como alternativa. | O usuário prefere ficar no Spotify; o Deezer volta à mesa se o Spike A falhar sem plano B viável. |
