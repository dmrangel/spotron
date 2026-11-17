# Player Spotify customizado (Windows)

Player desktop leve para Windows que toca músicas do Spotify (conta Premium) com DSP próprio (EQ paramétrico, compressor, crossfeed, presets AutoEQ) e interface totalmente customizável.

> **Aviso:** este projeto usa o librespot, uma implementação não oficial do protocolo do Spotify. Isso viola os Termos de Uso do Spotify. Uso pessoal apenas, sem distribuição e sem nenhuma função de download. O Spotify pode quebrar a compatibilidade a qualquer momento. Mantenha o app oficial instalado como reserva.

---

## Objetivos

1. **Leveza:** uso de memória muito menor que o app oficial (meta: < 150 MB com a UI aberta).
2. **Som:** EQ paramétrico multibanda, pré-amplificação, compressor/limitador, crossfeed para fones, importação de presets AutoEQ.
3. **Visual:** temas customizáveis (CSS), visualizador de áudio em tempo real.
4. **Portabilidade futura:** o formato de presets de DSP deve ser independente da plataforma, para ser reaproveitado num futuro projeto Android separado.

## Fora do escopo

- Android (projeto separado no futuro).
- Lossless: o Spotify não entrega FLAC via protocolo Connect para clientes de terceiros. Teto de qualidade: Ogg Vorbis 320 kbps.
- Download e cache offline de músicas.
- Distribuição pública.

---

## Arquitetura

```
┌───────────────────────── App Tauri 2 ─────────────────────────┐
│                                                                │
│  Frontend (WebView2)            Backend Rust                   │
│  ┌──────────────────┐          ┌───────────────────────────┐   │
│  │ UI + temas CSS   │◄─events─┤ librespot (crate)          │   │
│  │ Editor de EQ     │──invoke─►  Session / Player          │   │
│  │ Visualizador     │          │  Sink custom → PCM         │   │
│  └──────────────────┘          │        │         │         │   │
│                                │        │      tap FFT ──────┼─► visualizador
│                                └────────┼──────────────────┘   │
│                                         ▼ stdin (PCM S16LE)    │
│                                ┌───────────────────────────┐   │
│                                │ camilladsp.exe (sidecar)  │   │
│                                │ capture: Stdin            │   │
│                                │ playback: WASAPI          │◄──┼── WebSocket (controle do DSP)
│                                └───────────────────────────┘   │
└────────────────────────────────────────────────────────────────┘
```

### Decisões e motivos

| Decisão | Motivo | Alternativa descartada |
|---|---|---|
| **Tauri 2** para a UI | Usa o WebView2 do sistema: binário pequeno, pouca RAM. Temas são só CSS. O backend já é Rust, a mesma linguagem do librespot. | Electron (o mesmo problema de peso do app oficial). UI nativa em Rust (egui/iced), mais difícil de tematizar. |
| **librespot como crate**, embutido no backend | Controle direto de playback (load, pause, seek) sem depender da Web API para tocar. Um processo a menos. | Binário `librespot.exe --backend pipe` como sidecar (aceitável na Fase 0 para validar). Spotifyd (sem suporte real a Windows/Android e sem ponto de inserção de DSP). |
| **CamillaDSP como sidecar** | Motor de DSP maduro: biquads, compressor, limiter, delay, convolução FIR e API WebSocket para mudar parâmetros em tempo real. | DSP próprio em Rust (fica para o projeto Android, que não pode usar o CamillaDSP). |
| **Sink custom** no librespot | Recebe o PCM decodificado, escreve no stdin do CamillaDSP e alimenta o tap do visualizador. | Saída direta do librespot (sem como inserir DSP). |
| **Presets em JSON próprio** | Formato portátil. O app converte JSON → YAML/config do CamillaDSP. O futuro app Android lê o mesmo JSON. | Salvar YAML do CamillaDSP direto (amarra o formato ao motor). |

---

## Estrutura do repositório (proposta)

```
/
├─ PLANO.md                  ← este arquivo
├─ CLAUDE.md                 ← instruções para sessões do Claude Code
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ spotify/            ← sessão, login OAuth, player, sink custom
│  │  ├─ dsp/                ← processo CamillaDSP, cliente WebSocket, conversor de presets
│  │  ├─ viz/                ← tap de PCM + FFT → eventos para o frontend
│  │  └─ commands.rs         ← comandos Tauri expostos à UI
│  ├─ binaries/              ← camilladsp-x86_64-pc-windows-msvc.exe (sidecar)
│  └─ tauri.conf.json
├─ src/                      ← frontend Vue 3 + Tailwind (mesma stack do Modrinth App)
│  ├─ themes/                ← temas CSS (modrinth-dark.css é o padrão)
│  ├─ tokens.css             ← camada de tokens semânticos do app
│  └─ components/
├─ presets/                  ← presets de EQ em JSON (formato próprio)
└─ spikes/                   ← scripts e configs da Fase 0
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

### Fase 0: Spike da cadeia de áudio (sem app, sem UI)

**Objetivo:** provar que `librespot → CamillaDSP → WASAPI` funciona nesta máquina Windows. Esta é a fase que pode inviabilizar o projeto, por isso vem primeiro.

1. Instalar o Rust (toolchain MSVC) e compilar o librespot 0.8.x (`cargo install librespot` ou build do repositório).
2. Baixar o binário do CamillaDSP para Windows.
3. Rodar o librespot como dispositivo Connect com `--backend pipe` (sem `--device`, saída no stdout) e fazer o pipe para o `camilladsp` com uma config usando `capture: Stdin` e `playback: Wasapi`.
4. Pelo app oficial do Spotify (celular ou desktop), selecionar o dispositivo librespot e tocar uma música.
5. Aplicar uma config com um filtro óbvio (ex.: passa-baixa em 500 Hz) e confirmar que o som muda.
6. Ligar o WebSocket do CamillaDSP e mudar o ganho de um filtro em tempo real (via `websocat` ou script Python).

**Critérios de aceite:**
- [ ] Música toca sem estalos por 10 minutos seguidos.
- [ ] Filtro aplicado é audível.
- [ ] Mudança via WebSocket acontece sem interromper a música.
- [ ] Formato de amostra documentado (taxa, bits, canais) e batendo entre librespot e CamillaDSP.

**Conferido no código-fonte em 17/11/2026 (librespot 0.8.0, CamillaDSP 4.1.3):**
- librespot: `--backend pipe` sem `--device` escreve no stdout; `--format S16` (padrão) = S16LE; taxa fixa em **44100 Hz, 2 canais** (`playback/src/lib.rs`). Logs vão para o stderr.
- CamillaDSP 4.1: `capture: {type: Stdin, format: S16_LE}`, `playback: {type: Wasapi, format: F32, exclusive: false}`. Alteração ao vivo via `{"SetConfigValue": [json_pointer, valor]}` no WebSocket (`-p PORTA`).
- **WASAPI shared exige `samplerate` = formato padrão do dispositivo.** A maioria dos dispositivos vem em 48 kHz, então o CamillaDSP precisa reamostrar 44,1 → 48 kHz (`resampler: Synchronous` + `capture_samplerate: 44100`). A alternativa é mudar o formato padrão do Windows para 44,1 kHz ou usar o modo exclusivo.
- O backend `subprocess` do librespot mata o processo filho a cada pause: **não serve** para o CamillaDSP. Fica valendo o `pipe` no spike e o sink custom na Fase 1+.
- Arquivos do spike em `spikes/fase0/`.

**Pontos a verificar (não assumir):**
- Flags exatas do librespot 0.8 para o backend pipe e formato de saída (`--format`).
- Sintaxe da config do CamillaDSP na versão atual (`Stdin`, `Wasapi`, nomes de campos).
- Se o Windows `cmd`/PowerShell faz o pipe binário sem corromper os dados. **PowerShell 5 corrompe pipes binários:** usar `cmd` ou PowerShell 7+, ou um script Python que faça a ponte.

### Fase 1: Esqueleto Tauri + librespot embutido

1. Criar o app Tauri 2.
2. Adicionar o librespot como dependência (crates `librespot-core`, `librespot-playback`, `librespot-connect`, `librespot-metadata`; confirmar os nomes e versões).
3. Implementar login via OAuth do librespot, com cache de credenciais em `%APPDATA%`.
4. Implementar o sink custom que, nesta fase, só envia o áudio para a saída padrão (sem DSP).
5. Expor ao frontend os comandos `play(uri)`, `pause`, `resume`, `next`, `prev` e `seek`.
6. Expor o app também como dispositivo Spotify Connect, controlável pelo celular.

**Critérios de aceite:**
- [ ] Login funciona e persiste entre reinícios.
- [ ] Tocar uma URI colada na UI funciona.
- [ ] Controles básicos funcionam.
- [ ] RAM do processo medida e anotada.

### Fase 2: DSP integrado

1. Tauri inicia o `camilladsp.exe` como sidecar ao abrir e encerra ao fechar.
2. O sink custom escreve o PCM no stdin do CamillaDSP.
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

**Critérios de aceite:**
- [ ] Mudar uma banda na UI altera o som em < 100 ms, sem cortes.
- [ ] Importar um preset AutoEQ real funciona.
- [ ] Fechar o app não deixa o `camilladsp.exe` órfão.

### Fase 3: Interface e navegação

1. Busca, biblioteca, playlists e fila.
2. **Verificar primeiro** quais dados vêm do próprio librespot (metadata via sessão) e quais exigem a Web API. O Spotify restringiu vários endpoints da Web API para apps novos (2024 e 2026). Preferir os dados via librespot quando possível.
3. Editor visual de EQ (curva de resposta desenhada a partir dos biquads, com pontos arrastáveis).
4. Sistema de temas: implementar a arquitetura de duas camadas da seção "Design system", começando pelos três temas Modrinth, com troca em tempo real.
5. Layout inspirado no Modrinth App: barra lateral de navegação fixa (surface-2), conteúdo em cards (surface-3) e barra de reprodução fixa embaixo.

### Fase 4: Visualizador e polimento

1. Tap no sink: copiar blocos de PCM para um ring buffer, calcular a FFT (crate `rustfft`) e emitir eventos Tauri a ~30–60 fps.
2. Visualizador no frontend (canvas/WebGL).
3. Integração com as mídias do Windows (SMTC: teclas de mídia e overlay de volume).
4. Bandeja do sistema, atalhos globais e inicialização com o Windows (opcional).

---

## Riscos conhecidos

| Risco | Impacto | Mitigação |
|---|---|---|
| O Spotify muda o protocolo e o librespot quebra | Player para de tocar por dias ou semanas | Fixar a versão do librespot, acompanhar as issues do repositório e manter o app oficial como reserva. |
| Conta sinalizada por uso de cliente não oficial | Perda de acesso | Uso pessoal, sem downloads, sem automação agressiva. Risco baixo, mas não zero. |
| Web API restrita para apps novos | Busca e biblioteca limitadas | Priorizar metadados via sessão do librespot e testar cedo na Fase 3. |
| Pipe e latência entre o sink e o CamillaDSP | Estalos, dessincronia do visualizador | Medir na Fase 0 e ajustar o `chunksize` do CamillaDSP. Visualizador lê o tap antes do DSP (ou aceita um pequeno atraso). |
| Mudança de dispositivo de áudio no Windows (fone conectado/desconectado) | CamillaDSP perde a saída | Detectar o erro e reiniciar o sidecar com o dispositivo novo. |

---

## Sugestão de CLAUDE.md

```markdown
# Contexto
Player Spotify para Windows: Tauri 2 + librespot (crate) + CamillaDSP (sidecar).
Plano completo em PLANO.md. Trabalhar uma fase por vez, respeitando os critérios de aceite.

# Regras
- Nunca implementar download ou cache offline de faixas.
- Antes de usar qualquer API do librespot ou config do CamillaDSP, conferir a versão
  instalada (docs.rs / repositório); as APIs mudam entre versões.
- Toolchain Rust: MSVC. Shell para pipes binários: cmd ou PowerShell 7+, nunca PowerShell 5.
- Presets de DSP ficam em JSON próprio (formato em PLANO.md), nunca YAML do CamillaDSP direto.
- Medir a RAM ao fim de cada fase e registrar em PLANO.md.
- Frontend: Vue 3 + Tailwind. Componentes usam só tokens --app-* (tokens.css), nunca cores fixas.
- Valores de tema vêm de packages/assets/styles/variables.scss do repositório modrinth/code.
  Copiar valores, não inventar. Ler o LICENSE do pacote antes de copiar arquivos inteiros.
  Nunca usar logo, nome ou ícones da marca Modrinth.
```

---

## Registro de decisões

| Data | Decisão | Motivo |
|---|---|---|
| 2026-11-17 | CamillaDSP fixado em 4.1.3 (5.0 ainda beta); librespot em 0.8.0. | Últimas estáveis. |
| 2026-11-17 | Reamostragem 44,1→48 kHz no CamillaDSP quando o dispositivo não estiver em 44,1 kHz. | WASAPI shared não aceita taxa diferente do formato padrão; o librespot é fixo em 44,1 kHz. |
| 2026-11-17 | Spike usa `--backend pipe`, não `subprocess`. | `subprocess` mata o filho no pause. |
