# [ARQUIVADO] Fase 0 com librespot: bloqueada pelo Spotify

> **Resultado (17/11/2026, Windows 11, librespot 0.8.0, CamillaDSP 4.1.3):** não toca.
> O librespot autentica (zeroconf, conta Premium BR) e recebe as faixas do celular, mas o Spotify
> recusa a chave de áudio em todas elas:
>
> ```
> ERROR librespot_core::audio_key] error audio key 0 1
> WARN  librespot_playback::player] Unable to load key, continuing without decryption: Service unavailable { audio key error }
> ERROR librespot_playback::player] Unable to read audio file: Symphonia Decoder Error: end of stream
> ERROR librespot_playback::player] Skipping to next track, unable to load track ...
> ```
>
> É a restrição do lado do Spotify descrita em [librespot#1649](https://github.com/librespot-org/librespot/issues/1649):
> segundo o README dos mantenedores, afeta todas as versões, sem workaround conhecido. O OAuth também falha.
> O lado do CamillaDSP funcionou: abriu "Fones de ouvido (Realtek(R) Audio)" em WASAPI shared, 48000 Hz F32,
> com o resampler 44100→48000 e o pipeline de filtros montado.
>
> O projeto trocou o librespot pelo Spotify Web Playback SDK (ver PLANO.md e `spikes/fase0/`).
> Os arquivos abaixo ficam como registro.

Objetivo original: provar `librespot → pipe → CamillaDSP → WASAPI` numa máquina Windows real.
Nada aqui roda no Linux/CI: precisa de placa de som, conta Premium e o app do Spotify no celular.

Versões alvo (conferidas em 17/11/2026):

| Componente | Versão | Fonte |
|---|---|---|
| librespot | 0.8.0 (crates.io, nov/2025) | `cargo install librespot --locked` |
| CamillaDSP | 4.1.3 (última estável; 5.0 ainda é beta) | `camilladsp-windows-amd64.zip` nos releases do GitHub |

## Passo a passo

1. **Rust MSVC**: instale o `rustup` e as "Build Tools for Visual Studio" (C++). O librespot 0.8 exige Rust ≥ 1.85.
2. **librespot**: `cargo install librespot --locked --version 0.8.0`. O build padrão traz `native-tls`, `rodio` e `libmdns`, sem CMake nem NASM.
3. **CamillaDSP**: baixe `camilladsp-windows-amd64.zip` da release v4.1.3 e coloque o `camilladsp.exe` nesta pasta ou no PATH. Todas as builds já trazem o servidor WebSocket.
4. **Taxa do dispositivo**: em Painel de Som > dispositivo > Propriedades > Avançado, veja o "Formato padrão".
   - 48000 Hz: use o `fase0.yml` como está (ele reamostra de 44100 para 48000).
   - 44100 Hz: edite o `fase0.yml` conforme o comentário do topo.
5. **Rodar**: abra o **cmd.exe** (não o PowerShell 5) nesta pasta e execute `run.cmd`.
   Aceite o prompt do Firewall do Windows: sem ele o mDNS não anuncia o dispositivo.
6. No app do Spotify (celular), escolha o dispositivo **"Spotron Fase0"** e toque algo.
   O passa-baixa de 500 Hz já vem ativo, então o som deve sair abafado. Esse é o teste do filtro audível.
7. Em outro terminal, rode `pip install websockets` e depois `python ws_test.py`.
   O script troca o passa-baixa entre 500 Hz e 20 kHz e varre um peaking em 1 kHz, mostrando a latência de cada comando.

## O que observar e anotar (critérios de aceite)

- [ ] 10 min sem estalos. Se estalar, suba `chunksize` para 2048 e depois 4096, e anote o valor que funcionou.
- [ ] Filtro audível (passo 6).
- [ ] Troca via WebSocket sem interromper a música (passo 7), com a latência anotada.
- [ ] Formato: o `librespot.log` deve mostrar `S16`, e o `camilladsp.log` deve mostrar capture 44100 / playback 48000 (ou 44100).

## Testes extras (riscos que o plano não cobria)

- **Pause longo**: pause por 1 minuto e retome. No backend `pipe`, o librespot simplesmente para de escrever no stdout. Verifique se o CamillaDSP sobrevive (procure underrun ou erro no `camilladsp.log`) e se o áudio volta sozinho.
- **Pular faixa / seek**: deve continuar sem estalos e sem reiniciar o processo.
- **Trocar a saída padrão** (ex.: plugar um fone): anote o que acontece com `device: null`.
- **RAM**: anote a memória do `librespot.exe` e do `camilladsp.exe` no Gerenciador de Tarefas.

Cole os resultados no "Registro de decisões" do `PLANO.md`.

## Por que `pipe` e não `subprocess`

O librespot 0.8 também tem `--backend subprocess --device "camilladsp.exe ..."`, que evita o pipe do shell.
Ele foi descartado: o `SubprocessSink::stop()` **mata o processo filho a cada pause** (`playback/src/audio_backend/subprocess.rs`).
Isso reiniciaria o CamillaDSP, reabriria o WASAPI e derrubaria o WebSocket a cada pause.
