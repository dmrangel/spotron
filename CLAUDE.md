# Contexto
Player Spotify para Windows: Tauri 2 (UI) + Spotify Web Playback SDK numa janela do Edge `--app`
com perfil próprio + Web API (OAuth PKCE) + CamillaDSP (sidecar, capturando do VB-CABLE).
Plano completo em PLANO.md. Fase 0 concluída em 17/11/2026.
O SDK NÃO roda no WebView2 do Tauri: o Spotify recusa a licença Widevine dele (HTTP 403).
Trabalhar uma fase por vez, respeitando os critérios de aceite.

# Regras
- Nunca implementar download, cache offline ou gravação de faixas. A captura do VB-CABLE é só
  para DSP e análise em memória (FFT); nada de áudio vai para disco.
- O librespot foi descartado (o Spotify recusa a chave de áudio desta conta, librespot#1649).
  Não reintroduzir sem evidência nova. Registro em spikes/arquivo/librespot/.
- Antes de usar qualquer endpoint da Web API, recurso do SDK ou config do CamillaDSP, conferir a
  documentação atual: o Spotify restringiu o modo de desenvolvimento em 2024 e 2026
  (busca máx. 10 por página; itens só de playlists próprias/colaborativas; endpoints removidos).
  Versão fixada: CamillaDSP 4.1.3 (não usar 5.0-beta).
- OAuth: Authorization Code com PKCE. Redirect em http://127.0.0.1:<porta>/callback
  (o Spotify não aceita `localhost`).
- Áudio: o msedge.exe do player toca no "CABLE Input (VB-Audio Virtual Cable)" (preferência por
  executável no Mixer de volume); o CamillaDSP captura "CABLE Output" via WASAPI shared com
  enable_rate_adjust + AsyncSinc e toca na saída padrão (`device: null`, que acompanha a troca de
  saída sozinho). Valores validados: chunksize 1024, target_level 1024 (spikes/fase0/vbcable.yml).
  WASAPI shared exige samplerate igual ao formato padrão de cada dispositivo (hoje 48000 Hz).
- Toolchain Rust: MSVC. Shell para pipes binários: cmd ou PowerShell 7+, nunca PowerShell 5.
- Presets de DSP ficam em JSON próprio (formato em PLANO.md), nunca YAML do CamillaDSP direto.
- Medir a RAM ao fim de cada fase e registrar em PLANO.md.
- Frontend: Vue 3 + Tailwind. Componentes usam só tokens --app-* (tokens.css), nunca cores fixas.
- Valores de tema vêm de packages/assets/styles/variables.scss do repositório modrinth/code.
  Copiar valores, não inventar. Ler o LICENSE do pacote antes de copiar arquivos inteiros.
  Nunca usar logo, nome ou ícones da marca Modrinth.
- Nunca commitar Client ID secreto, tokens ou refresh tokens (o fluxo PKCE não usa client secret).

# Ambiente
- Sessões na nuvem (Linux) não têm áudio, Windows, Widevine nem VB-CABLE: lá só se escreve e
  compila código. Os testes de áudio rodam na máquina Windows do usuário.
- Na máquina do usuário: Rust 1.100 MSVC, Build Tools 2022, Python 3.14 (pip global quebrado ao
  gerar launchers: usar o .venv em spikes/fase0/).
