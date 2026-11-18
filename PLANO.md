# Player Spotify customizado (Windows)

Player desktop leve para Windows que toca músicas do Spotify (conta Premium) com DSP próprio (EQ paramétrico, compressor, crossfeed, presets AutoEQ) e interface totalmente customizável, sem depender do app oficial do Spotify.

> **Aviso:** o app usa o **Spotify Web Playback SDK** e a **Web API**, que são oficiais, num app de desenvolvedor em modo de desenvolvimento (uso pessoal). O áudio do SDK sai pelo Windows e é capturado de um cabo virtual para aplicar o DSP em tempo real. As regras de desenvolvedor do Spotify proíbem gravar, armazenar ou fazer "ripping" do conteúdo (o app **nunca** grava nada) e trazem uma cláusula genérica contra "modificar/alterar" o conteúdo. EQ em tempo real não é citado diretamente: é uma zona cinzenta. Uso pessoal apenas, sem distribuição, sem uso comercial e sem nenhuma função de download ou gravação.

---

## Objetivos

1. **Estabilidade e leveza:** substituir o app oficial (que trava nesta máquina) por um app próprio. Meta de memória: < 250 MB com a UI aberta e tocando (inclui o processo Widevine do WebView2, ~55 MB).
2. **Som:** os efeitos do Neutron Music Player: EQ paramétrico e gráfico, correção AutoEQ, pré-amplificação, compressor/limitador, crossfeed para fones, Ambiophonic R.A.C.E. para caixas, loudness, alinhamento de tempo e outros (lista em "Efeitos de DSP").
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
│  Frontend (WebView2)                     Backend Rust                │
│  ┌────────────────────────────┐         ┌──────────────────────────┐ │
│  │ UI + temas CSS             │◄─events─┤ OAuth PKCE + tokens      │ │
│  │ Editor de EQ, Visualizador │─invoke─►│ Cliente Web API          │ │
│  │ Controles de reprodução    │         │ Servidor local (página   │ │
│  └────────────────────────────┘         │  do player + /token)     │ │
│                                         │ Processo Edge --app      │ │
│                                         │ Processo CamillaDSP      │ │
│                                         │ Cliente WebSocket (DSP)  │ │
│                                         │ Captura p/ FFT ──────────┼─► visualizador
│                                         └────────────┬─────────────┘ │
└──────────────────────────────────────────────────────┼───────────────┘
                                                       │ inicia/encerra
        ┌──────────────────────────────────────────────▼───┐
        │ msedge.exe --app=http://127.0.0.1:<porta>/        │
        │ perfil próprio, janela oculta/minimizada          │
        │ Spotify Web Playback SDK (Widevine, AAC 256)      │
        └──────────────────────────┬───────────────────────┘
                                   ▼  saída do msedge.exe → "CABLE Input (VB-Audio Virtual Cable)"
        ┌───────────────────────────────────┐
        │ camilladsp.exe (sidecar)          │◄── WebSocket (controle do DSP)
        │ capture: Wasapi "CABLE Output"    │
        │ rate adjust + AsyncSinc           │
        │ playback: Wasapi (padrão)         │
        └───────────────────────────────────┘
```

O áudio do SDK é protegido por DRM e é tocado pelo próprio Edge: o código do app não tem acesso ao PCM. O único ponto de inserção do DSP é depois da saída do Windows. Por isso o player toca num **cabo de áudio virtual** e o CamillaDSP captura dele. O SDK roda no Edge e não no WebView2 do Tauri porque o Spotify recusa a licença do WebView2 (Spike A).

### Decisões e motivos

| Decisão | Motivo | Alternativa descartada |
|---|---|---|
| **Tauri 2** para a UI | WebView2 do sistema: binário pequeno, pouca RAM, temas em CSS. | Electron (mais pesado; o castLabs ECS exige assinatura VMP via EVS a cada build). |
| **Spotify Web Playback SDK** para tocar | Oficial, toca com a conta do usuário. Aparece como dispositivo Connect, controlável também pelo celular. | **librespot**: o Spotify recusa a chave de áudio desta conta (`audio key error 0 1`, librespot#1649, sem workaround). Testado em 17/11/2026. |
| **Edge em modo `--app`** (perfil próprio) como janela de reprodução do SDK | O Spotify aceita a licença Widevine do Edge (e do Chrome); o Edge existe em todo Windows e não é o navegador de uso do usuário, então a preferência de saída por executável não atrapalha. O Tauri sobe uma página local que hospeda o SDK. | **SDK dentro do WebView2 do Tauri**: Widevine existe, mas a licença volta HTTP 403 (13/14). Chrome `--app` funciona, mas não existe em todo Windows. |
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
│  ├─ player.ts              ← estado do player e comandos (a página do SDK fica em src-tauri/player/)
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
- [x] Widevine disponível no WebView2 (`requestMediaKeySystemAccess` resolve). *17/11/2026: OK, Edge/WebView2 157, `com.widevine.alpha` SW_SECURE_CRYPTO, sem configuração.*
- [x] Toca 10 minutos seguidos, com troca de faixa e seek, sem erro de licença.
  *17/11/2026, **Chrome `--app` via `player.py`: aprovado.** Contador em 10 min tocando (~12 min de sessão), 0 erros, 3 idas e voltas da reprodução entre o Spotron e outros dispositivos (celular, iPad, PC), troca de faixa automática OK.*
  *17/11/2026, **WebView2: falhou.** O servidor de licença do Spotify (`/v1/widevine-license/v1/audio/license`) respondeu HTTP 403 em 13 de 14 pedidos; as faixas pulam com `playback_error`. O SDK carrega, o CDN entrega o áudio, só a licença é recusada (provável falta de assinatura VMP aceita pelo Spotify no WebView2).*
  ***Chrome `--app`: funcionou.** 4 de 4 licenças HTTP 200, 0 `playback_error`, troca de faixa, voltar e seek OK, qualidade boa segundo o usuário. Tocou ~4 min contínuos (o usuário encerrou antes dos 10 min).*
- [x] Plano B validado: Chrome em `--app=http://127.0.0.1:8890/` com `--user-data-dir` próprio (`sdk/browser_serve.py`). Edge `--app` ainda não testado.
- RAM do Chrome `--app` tocando (10 processos): **427 MB privados** (777 MB de working set). Maiores: GPU 222 MB, browser 163 MB, CDM 103 MB. Acima da meta; testar `--disable-gpu` e outras flags na Fase 1.

#### Spike B: VB-CABLE → CamillaDSP → fones

1. Rotear a saída do app para "CABLE Input" (Configurações > Sistema > Som > Mixer de volume > app > Dispositivo de saída). **Verificar** qual processo aparece no mixer: o áudio do WebView2 sai de um `msedgewebview2.exe`, que pode ser compartilhado com outros apps (Teams, Widgets). Se for, testar o WebView2 *Fixed Version* empacotado com o app (caminho de executável próprio).
2. Rodar `camilladsp.exe -p 1234 -v vbcable.yml` (captura "CABLE Output", playback no dispositivo padrão, `enable_rate_adjust` + `AsyncSinc`).
3. Repetir o `ws_test.py` (passa-baixa 500 Hz ↔ 20 kHz, varredura do peaking).

**Critérios de aceite:**
*Resultados de 17/11/2026 (player no Edge `--app`, janela cronometrada 21:24–21:34):*
- [x] 10 min sem estalos com **`chunksize: 1024`, `target_level: 1024`** (sem precisar subir). 0 underrun/overrun com música tocando, 0 erros, 0 amostras clipadas. O usuário confirmou "perfeito" e boa definição.
- [x] Rate adjust estável: taxa medida do cabo oscila 47 611–48 260 Hz por segundo (média ~48 110 Hz); fator de correção entre 0,99975 e 1,00008; buffer ~1 100–2 300 amostras. Os 3 "underruns" da janela vêm todos do `silence_timeout` (pause do usuário e saída da reprodução para outro dispositivo) e são reinícios esperados, não estalos.
- [x] Passa-baixa de 500 Hz audível (confirmado pelo usuário).
- [x] Mudança via WebSocket sem interromper a música: **0,1–1,5 ms** por comando `SetConfigValue` (o som muda no bloco seguinte, ~21 ms). Carga de processamento ~0,1–0,3 %.
- [x] Só o áudio do app passa pelo cabo: a preferência do Mixer de volume é **por executável** (`msedge.exe`). Ok porque o usuário não usa o Edge no dia a dia; em geral, qualquer uso do Edge passaria pelo DSP (ver Riscos).
- [x] Formato: AAC 256 (SDK) → Edge → "CABLE Input" 48000 Hz → CamillaDSP F32 48000 Hz (AsyncSinc Balanced + rate adjust) → "Fones de ouvido (Realtek)" 48000 Hz F32 shared.

*Testes extras:*
- Pause de ~20 s e retomada: OK (o CamillaDSP pausa no silêncio e retoma sozinho).
- Reprodução levada para outro dispositivo e trazida de volta: OK.
- **Troca da saída padrão do Windows (fones → alto-falantes): OK, sem nenhum evento no log.** Com `device: null` o Windows move o fluxo sozinho (roteamento automático do WASAPI shared); não precisou reiniciar o CamillaDSP.
- Mesmo teste com o Chrome `--app` antes da troca para o Edge: também OK.

#### Spike C: OAuth PKCE + Web API

1. Fluxo Authorization Code com PKCE, redirect em `http://127.0.0.1:8888/callback`, escopos `streaming user-read-email user-read-private user-read-playback-state user-modify-playback-state playlist-read-private playlist-read-collaborative user-library-read`.
2. Refresh token salvo e renovado.
3. Chamar: `GET /search` (máx. 10 por página), `GET /me/playlists`, `GET /playlists/{id}/items`, `GET /me/tracks`, `POST /me/player/queue`, `PUT /me/player` (transferir para o dispositivo do SDK).

**Critérios de aceite:**
- [x] Login funciona e o refresh token renova sem novo login.
- [x] Busca, playlists próprias, curtidas e fila respondem.
- [x] Anotar o que falha: só `limit` > 10 na busca (HTTP 400 "Invalid limit"). Itens de playlist seguida mas não própria vieram (217 itens), ao contrário do esperado.

**RAM ao fim da Fase 0 (17/11/2026, tocando):**
- `camilladsp.exe`: **5 MB** privados (21 MB working set).
- Player Edge `--app` (12 processos): **444 MB** privados (804 MB working set). Maiores: GPU 146 MB, renderer 83 MB, CDM (Widevine) 68 MB, browser 56 MB. Inclui serviços inúteis aqui (indexador de busca, carteira).
- Player Chrome `--app` (10 processos), para comparação: 427 MB privados.
- Meta de < 250 MB ainda não atingida: na Fase 1 testar `--disable-gpu`, `--disable-features=...` para os serviços extras do Edge e medir de novo.

**Fase 0 concluída em 17/11/2026:** Spikes A (Edge/Chrome `--app`), B e C aprovados.

### Fase 1: Esqueleto Tauri + player

1. Criar o app Tauri 2 de verdade (Vue 3 + Tailwind).
2. Login OAuth PKCE no backend Rust, com tokens em `%APPDATA%` (refresh automático).
3. Player do SDK: o backend sobe o servidor local (página do player + `/token`, como `spikes/fase0/sdk/player.py`) e abre o `msedge.exe --app` com perfil próprio, minimizado ou fora da tela; encerra o Edge junto com o app. Reaproveitar a robustez do spike (token com novas tentativas, reconexão, início sem clique com `--autoplay-policy=no-user-gesture-required`).
4. Expor ao frontend `play(uri)`, `pause`, `resume`, `next`, `prev`, `seek`, `volume`, via Web API (e mensagens para a página do player quando a Web API não bastar).
5. O app aparece como dispositivo Spotify Connect, controlável também pelo celular.
6. ~~Reduzir a RAM do Edge e medir.~~ **Feito em 17/11/2026** (`spikes/fase0/sdk/ram_bench.py`): usar `OPTIMIZE_FLAGS` do `player.py` (sem GPU, sem serviços extras do Edge, sem isolamento de sites). Edge tocando: **403 → ~213–216 MB** privados (9 processos), 0 erros, som confirmado sem estalos pelo usuário. Total com o CamillaDSP (5 MB): ~221 MB, abaixo da meta de 250 MB (ainda falta somar o Tauri).

   | Flags | RAM privada | Tocou |
   |---|---|---|
   | nenhuma | 403 MB | sim |
   | sem GPU | 256 MB | sim |
   | + sem serviços extras | 251–258 MB | sim |
   | **+ sem isolamento de sites** | **213 MB** | sim |
   | + 1 renderer + heap JS 96 MB | 211–215 MB | sim |
   | sem GPU + extras + só 1 renderer | 203–235 MB | inconclusivo (2×), descartado |

**Critérios de aceite** (17/11/2026, app em `src/` + `src-tauri/`, página do player em `src-tauri/player/player.html`):
- [x] Login funciona e persiste entre reinícios (Client ID e tokens em `%APPDATA%\Spotron`; o app foi reaberto várias vezes sem pedir login).
- [x] Tocar uma URI colada na UI funciona (links `open.spotify.com/...` e URIs `spotify:` de faixa, álbum, playlist, artista, episódio).
- [x] Controles básicos funcionam (play/pause, anterior/próxima, seek, volume; confirmado pelo usuário). O app aparece como "Spotron" no celular.
- [x] RAM medida (release, tocando): **373 MB privados** no total = `spotron.exe` 6 MB + WebView2 da UI 158 MB (6 processos) + Edge do player 204 MB (8) + CamillaDSP 5 MB. Acima da meta de 250 MB: próximo corte é o WebView2 da UI (testar `--disable-gpu` e afins via `additionalBrowserArgs`).

*Bug encontrado e corrigido:* com a janela do Edge fora da tela (`--window-position=-32000,-32000`) a página ficava `hidden` e o SDK aceitava a transferência mas não tocava (posição parada, sem pedir licença, sem erro). Correção: `--disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-background-timer-throttling` e `CalculateNativeWinOcclusion` em `--disable-features`. Diagnóstico com `SPOTRON_EDGE_DEBUG=1` (DevTools em 9223) + `spikes/fase0/sdk/cdp_watch.py 9223`.
*Job Object verificado:* matar o `spotron.exe` derruba todos os processos do Edge do player (0 órfãos).

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
     "crossfeed": { "enabled": false, "cutoff_hz": 700, "level_db": 4.5 },
     "ambiophonics": { "enabled": false, "delay_us": 70, "attenuation_db": 3.0, "low_hz": 250, "high_hz": 5000 }
   }
   ```
   O formato cresce com os efeitos da seção "Efeitos de DSP (paridade com o Neutron)".
5. Conversor preset JSON → config do CamillaDSP.
6. Importador do formato paramétrico do AutoEQ (`Filter N: ON PK Fc X Hz Gain Y dB Q Z`).
7. Proteção contra clipping: limiter no fim da cadeia, ligado por padrão.
8. Reiniciar o CamillaDSP quando o dispositivo de saída mudar (fone conectado/desconectado).

**Critérios de aceite:**
- [ ] Mudar uma banda na UI altera o som em < 100 ms, sem cortes.
- [ ] Importar um preset AutoEQ real funciona.
- [ ] Fechar o app não deixa o `camilladsp.exe` órfão.
- [ ] Trocar o fone não deixa o app mudo.

### Efeitos de DSP (paridade com o Neutron)

Pedido do usuário (17/11/2026): oferecer os efeitos do Neutron Music Player. Lista levantada em
neutroncode.com/apps/player, nas lojas e em reviews (vários parâmetros do Neutron não são públicos:
conferir no app antes de copiar faixas de valores). **Os nomes de filtros do CamillaDSP abaixo
precisam ser conferidos na 4.1.3 antes de implementar** (regra do CLAUDE.md).

Todos os efeitos são opções do preset JSON, com liga/desliga, e viram blocos do pipeline do
CamillaDSP. Ordem sugerida: correção (AutoEQ) → EQ do usuário → efeitos espaciais → dinâmica →
volume/loudness → limiter.

| Efeito (nome no Neutron) | O que faz | Como fazer no CamillaDSP | Fase |
|---|---|---|---|
| Parametric Equalizer | EQ por banda (até 130 bandas no Neutron), por canal | `Biquad` (Peaking, Lowshelf, Highshelf, Lowpass, Highpass, Bandpass, Notch, Allpass) e `BiquadCombo` (Butterworth/Linkwitz-Riley) | 2 |
| Graphic EQ | EQ de bandas fixas com curvas prontas | `BiquadCombo` GraphicEqualizer (conferir) ou N `Peaking` fixos | 2 |
| Frequency Response Correction | Correção AutoEQ de fones (e curvas do usuário para caixas) | Importador AutoEQ → `Biquad`; preset separado do EQ do usuário, os dois ativos ao mesmo tempo | 2 |
| Preamp | Ganho antes do DSP (headroom) | `Gain` | 2 |
| Compressor / Limiter | Compressão de dinâmica e teto de pico | Processor `Compressor`; filtro `Limiter` no fim (já ligado por padrão) | 2 |
| Crossfeed (fones) | Mistura parte de um canal no outro, filtrado e atrasado (estilo bs2b: 700 Hz/4,5 dB, Chu Moy 700 Hz/6 dB, Jan Meier 650 Hz/9,5 dB) | `Mixer` + `Biquad` Lowpass + `Delay` no sinal cruzado | 2 |
| **Ambiophonic R.A.C.E.** (caixas) | Cancelamento de crosstalk recursivo: cada canal recebe o oposto invertido, atrasado (60–120 µs, ~70 µs típico) e atenuado (2–4 dB), repetidamente; só na faixa ~250 Hz–5 kHz (graves e agudos passam direto) | O pipeline do CamillaDSP não tem realimentação: calcular a recursão truncada como 4 FIR curtos (L→L, R→L, L→R, R→R) e aplicar com `Mixer` + `Conv` (coeficientes gerados pelo app); banda limitada por crossover `BiquadCombo` | 2 |
| Surround x Crossfeed | No Neutron um exclui o outro | Mesma regra na UI: R.A.C.E. para caixas, crossfeed para fones | 2 |
| Adaptive Loudness Compensation | Compensação de loudness que depende do volume | Filtro `Loudness` ligado ao volume do CamillaDSP. **Exige que o volume do app seja o do CamillaDSP** (SDK fixo em 100%) | 2 |
| Time Delay Alignment | Atraso (e atenuação) por canal para caixas a distâncias diferentes | `Delay` (ms ou amostras) + `Gain` por canal | 2 |
| Speaker protection | Corte subsônico e ultrassônico | `Biquad` Highpass/Lowpass (ex.: Butterworth) | 2 |
| Phase/Polarity inversion | Inverte a fase de um canal ou de todos | `Gain` com `inverted: true` | 2 |
| Balance, Mono, troca L/R | Balanço, soma mono, inversão de canais | `Mixer` | 2 |
| Pseudo-stereo | Estéreo artificial para faixas mono | `Mixer` + `Delay`/Allpass para descorrelacionar | 4 (baixa prioridade) |
| Reverberator (I3DL2) | Reverb com presets | Não existe no CamillaDSP: aproximar com `Conv` de respostas ao impulso (IR) prontas | 4 (opcional) |
| Dithering | Reduz erro de quantização | Filtro `Dither`; só faz sentido com saída inteira (WASAPI exclusivo S16/S24). Em shared (F32) fica desligado | 4 (opcional) |
| Oversampling / resampler | Qualidade da reamostragem | Perfil do `AsyncSinc` (VeryFast…Accurate) nas configurações | 2 |
| Pitch e Tempo | Muda velocidade/tom | **Não dá**: a velocidade é do SDK do Spotify e o CamillaDSP não faz time-stretch | fora |

Não são DSP no nosso caso: gapless e crossfade (controlados pelo Spotify), ReplayGain/normalização
(o Spotify normaliza no servidor; conferir se o SDK expõe), bit-perfect, DSD, Chromecast/UPnP
(fora do escopo). Analisadores de espectro/RMS entram no visualizador da Fase 4.

**Critérios de aceite dos efeitos (junto com a Fase 2):**
- [ ] Cada efeito liga/desliga ao vivo via WebSocket, sem cortar a música.
- [ ] R.A.C.E.: com sinal só no canal esquerdo, ajustar delay/atenuação deixa a imagem toda à esquerda (método de calibração de Glasgal); sem clipping com o limiter ligado.
- [ ] Crossfeed: os três presets bs2b audíveis e sem clipping.
- [ ] Loudness acompanha o volume do app.

Fontes: [neutroncode.com/apps/player](https://neutroncode.com/apps/player),
[motor de áudio do Neutron](https://neutroncode.com/technology/79-audio-engine),
[Glasgal & Miller, RACE (2007)](https://filmaker.com/papers/RGRM-RACE_rev.pdf),
[bs2b](https://bs2b.sourceforge.net/).

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
| **Limite no servidor de licença** (visto em 17/11/2026, ~22:30) | Depois de centenas de licenças em poucas horas (benchmarks, testes de pulo, muitos perfis/dispositivos novos), `widevine-license` passou a responder 403 sem corpo para o nosso player (Edge, qualquer flag), e o SDK pula faixas. A Web API seguia OK (200) e o open.spotify.com/app oficial tocavam normalmente. | Testes de reprodução econômicos (1 dispositivo, poucas faixas; nada de benchmarks em laço). Não contornar com outro Client ID. Retestar após pausa; no app, evitar reconexões/reinícios que gerem licenças à toa. Testar o DSP com outro player (Opera → CABLE Input) enquanto isso. |
| Termos de desenvolvedor: "modificar/alterar" e "capturar" o stream | App de desenvolvedor suspenso | Uso pessoal, nada gravado, nada distribuído. Risco baixo, mas não zero. |
| O roteamento por app é por executável (`msedge.exe`) | Se o usuário abrir o Edge para navegar, esse áudio também passa pelo DSP | Aceito (o usuário usa o Opera). Alternativas se incomodar: `setSinkId` injetado no iframe do SDK via CDP, ou Chrome `--app`. |
| ~~Clock do VB-CABLE diferente do da placa~~ | ~~Estalos periódicos~~ | **Resolvido no Spike B:** rate adjust + AsyncSinc mantém 0 underrun com música (fator 0,99975–1,00008). |
| ~~Mudança de dispositivo de saída~~ | ~~CamillaDSP perde a saída~~ | **Resolvido no Spike B:** com `device: null` o Windows move o fluxo sozinho. Continua valendo detectar erro e reiniciar o sidecar como rede de segurança (ex.: dispositivo removido em modo exclusivo). |
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
| 2026-11-17 | ~~SDK tocando dentro do WebView2 do Tauri.~~ **Descartado.** | Widevine existe no WebView2 157, mas o Spotify responde 403 na licença (13/14). |
| 2026-11-17 | **Tauri 2 para a UI + Chrome em modo `--app` (perfil próprio) para tocar o SDK.** | No Chrome a licença passa (4/4 HTTP 200, 0 erros, troca de faixa e seek OK). O Tauri controla a janela de reprodução via página local e Web API. Custo: ~430 MB privados, a otimizar; testar Edge `--app` como alternativa já instalada em todo Windows. |
| 2026-11-17 | Spike C aprovado: OAuth PKCE, refresh e Web API. | 11/11 chamadas OK. Busca limitada a 10 por página (`limit=20` → 400), `offset` pagina. Itens de playlist seguida (não própria) vieram normalmente. Python precisa do `truststore` (CA "Certainly" do accounts.spotify.com). |
| 2026-11-17 | **Player do SDK no Edge `--app`** (antes Chrome). | Pedido do usuário: o Edge existe em todo Windows. Licença 200 em todos os pedidos, 10 min sem erro. |
| 2026-11-17 | **Spike B aprovado; Fase 0 concluída.** `chunksize` 1024 / `target_level` 1024; WebSocket 0,1–1,5 ms; 0 clipping; troca de saída transparente com `device: null`. RAM: CamillaDSP 5 MB, Edge 444 MB. | Medido em 10 min cronometrados com música, pause, troca de dispositivo e de saída. |
| 2026-11-17 | Edge do player com `--disable-gpu`, serviços extras desligados e `--disable-site-isolation-trials`. | 403 → 213 MB. A GPU sozinha era 161 MB (a janela não tem vídeo). Sem isolamento é aceitável porque o perfil só abre a página local com o iframe do Spotify. |
| 2026-11-17 | **Fase 1 concluída.** Edge do player fora da tela + flags anti-oclusão; Job Object (KILL_ON_JOB_CLOSE) prende o Edge ao app; controle da reprodução pela Web API com `device_id`. RAM total 373 MB. | Sem as flags anti-oclusão o SDK não toca com a janela fora da tela. |
| 2026-11-17 | Efeitos do Neutron entram no plano (Fase 2, alguns na 4), incluindo o Ambiophonic R.A.C.E. pedido como "AmbioRace". Pitch/tempo fica fora. | Pedido do usuário. R.A.C.E. vira FIR truncado (o CamillaDSP não tem realimentação no pipeline). |
| 2026-11-17 | Deezer (pleezer) considerado e deixado como alternativa. | O usuário prefere ficar no Spotify; o Deezer volta à mesa se o Spike A falhar sem plano B viável. |
