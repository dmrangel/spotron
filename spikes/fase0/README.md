# Fase 0: spikes da cadeia (Web Playback SDK → VB-CABLE → CamillaDSP)

Objetivo: provar nesta máquina Windows as três peças que podem inviabilizar o projeto.
Critérios de aceite completos em `PLANO.md` (Fase 0). A tentativa anterior com o librespot
está em `spikes/arquivo/librespot/` (bloqueada pelo Spotify).

Nada aqui roda no Linux/CI: precisa de Windows, Widevine, VB-CABLE e conta Premium.

## Pré-requisitos (feitos pelo usuário)

1. **App de desenvolvedor do Spotify** em <https://developer.spotify.com/dashboard>, com a conta Premium:
   - Redirect URI: `http://127.0.0.1:8888/callback` (o Spotify não aceita `localhost`).
   - APIs: Web API e Web Playback SDK.
   - User Management: adicionar a própria conta.
   - Anotar só o **Client ID** (o fluxo PKCE não usa o client secret; não commitar nada disso).
2. **VB-CABLE** (<https://vb-audio.com/Cable/>): instalar como administrador e reiniciar.
   No Painel de Som, pôr "CABLE Input" (Reprodução) e "CABLE Output" (Gravação) em
   **48000 Hz, 2 canais** (Propriedades > Avançado). Manter os fones como saída padrão.
3. Já prontos nesta máquina: `camilladsp.exe` 4.1.3 nesta pasta e `.venv` com `websockets`.

## Spike A: Web Playback SDK no WebView2 (Tauri mínimo)

Código a escrever na próxima etapa (`spikes/fase0/sdk/`): página Tauri mínima que carrega o SDK,
mostra o resultado de `requestMediaKeySystemAccess('com.widevine.alpha')` e loga os erros do SDK.

Teste: tocar pelo celular (dispositivo **"Spotron Fase0"**) por 10 minutos, com troca de faixa e seek.

## Spike B: VB-CABLE → CamillaDSP → fones

1. Com o app do Spike A tocando, abrir Configurações > Sistema > Som > Mixer de volume e pôr a
   saída do app em **"CABLE Input (VB-Audio Virtual Cable)"**. Anotar qual processo aparece
   (provavelmente `msedgewebview2.exe`) e se outros apps foram afetados.
   Para um teste antes do Spike A: rotear o navegador (Opera) tocando o open.spotify.com.
2. No **cmd.exe**, nesta pasta: `run_camilladsp.cmd`.
   O passa-baixa de 500 Hz já vem ativo: o som deve sair abafado.
3. Em outro terminal, nesta pasta: `.venv\Scripts\python.exe ws_test.py`.
4. Deixar tocando 10 minutos e conferir o `camilladsp.log` (underrun, overrun, taxa de captura medida).
   Se estalar, subir `chunksize`/`target_level` para 2048 e depois 4096.

## Spike C: OAuth PKCE + Web API

Código a escrever na próxima etapa (`spikes/fase0/api/`): login PKCE com redirect em
`http://127.0.0.1:8888/callback`, refresh token, e chamadas a `/search`, `/me/playlists`,
`/playlists/{id}/items`, `/me/tracks`, `/me/player/queue` e `PUT /me/player`.

## Anotar (colar no "Registro de decisões" do PLANO.md)

- Widevine no WebView2: sim/não; erros do SDK, se houver.
- `chunksize` e `target_level` que funcionaram; taxa de captura medida pelo rate adjust.
- Latência dos comandos WebSocket (impressa pelo `ws_test.py`).
- Processo que aparece no mixer e se o roteamento por app persiste após reiniciar.
- Endpoints da Web API que falharam.
- RAM: processos do WebView2 (incluindo o utilitário Widevine) e `camilladsp.exe`.
