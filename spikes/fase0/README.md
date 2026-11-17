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

Ordem: primeiro o Spike C (`login`), porque o Spike A usa o token que ele salva.

## Spike C: OAuth PKCE + Web API (`api/api.py`, só Python padrão)

No cmd, em `spikes\fase0\api`:

```
python api.py login --client-id <CLIENT_ID>   (abre o navegador; salva Client ID e tokens em %APPDATA%\spotron-spike)
python api.py test                            (testes só de leitura: /me, busca, playlists, curtidas, player)
python api.py token                           (renova o access token; rodar antes do Spike A se passou 1 h)
python api.py test --write                    (com o Spike A aberto: transfere para "Spotron Fase0" e põe 1 faixa na fila)
```

O `test` inclui de propósito uma busca com `limit=20` (deve falhar: o máximo é 10 desde fev/2026) e
`/items` de uma playlist seguida mas não própria (deve vir restrito). O resumo no fim lista o que falhou.

## Spike A: Web Playback SDK no WebView2 (`sdk/`, Tauri 2 mínimo)

Compilar (uma vez, ~7 min) e abrir, em `spikes\fase0\sdk`:

```
cargo build --release --features custom-protocol
target\release\spotron-fase0-sdk.exe
```

A página testa o Widevine ao abrir. Depois: **1. Iniciar player** → **2. Transferir para cá**
(ou escolher "Spotron Fase0" no celular). Tudo o que a página registra vai para `sdk\spike_a.log`.
Deixar tocando 10 minutos, com troca de faixa e seek; o contador mostra o tempo tocando e os erros.

Resultado (17/11/2026): o Widevine existe no WebView2 157, mas **o Spotify recusa a licença**
(HTTP 403 em 13/14 pedidos): as faixas pulam com `playback_error`.

### Plano B: a mesma página no Chrome em modo app (funcionou)

```
..\.venv\Scripts\python.exe player.py          (Chrome; --edge para o Edge; --debug liga o CDP em 9222)
```

O `player.py` sobe o servidor local (página + token) e abre o Chrome em `--app` com perfil próprio
e `--autoplay-policy=no-user-gesture-required`. O player inicia sozinho, o servidor vive enquanto a
janela estiver aberta, o token é pedido com novas tentativas e o player reconecta em `not_ready`
ou `authentication_error`: dá para levar a reprodução para outro dispositivo e trazer de volta
quantas vezes quiser.

Licença 4/4 HTTP 200, 0 erros, troca de faixa e seek OK. RAM: ~427 MB privados.

Diagnóstico de rede/console (WebView2 ou Chrome) com `cdp_watch.py`: abrir o alvo com
`--remote-debugging-port=9222` (no WebView2, via `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`).

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

## Anotar (colar no "Registro de decisões" do PLANO.md)

- Widevine no WebView2: sim/não; erros do SDK, se houver.
- `chunksize` e `target_level` que funcionaram; taxa de captura medida pelo rate adjust.
- Latência dos comandos WebSocket (impressa pelo `ws_test.py`).
- Processo que aparece no mixer e se o roteamento por app persiste após reiniciar.
- Endpoints da Web API que falharam.
- RAM: processos do WebView2 (incluindo o utilitário Widevine) e `camilladsp.exe`.
