# Contexto
Player Spotify para Windows: Tauri 2 + librespot (crate) + CamillaDSP (sidecar).
Plano completo em PLANO.md. Trabalhar uma fase por vez, respeitando os critérios de aceite.

# Regras
- Nunca implementar download ou cache offline de faixas.
- Antes de usar qualquer API do librespot ou config do CamillaDSP, conferir a versão
  instalada (docs.rs / repositório); as APIs mudam entre versões.
  Versões fixadas hoje: librespot 0.8.0, CamillaDSP 4.1.3 (não usar 5.0-beta).
- Toolchain Rust: MSVC. Shell para pipes binários: cmd ou PowerShell 7+, nunca PowerShell 5.
- O librespot gera sempre 44100 Hz / 2 canais. WASAPI shared exige `samplerate` igual ao
  formato padrão do dispositivo: reamostrar no CamillaDSP quando não for 44100.
- Presets de DSP ficam em JSON próprio (formato em PLANO.md), nunca YAML do CamillaDSP direto.
- Medir a RAM ao fim de cada fase e registrar em PLANO.md.
- Frontend: Vue 3 + Tailwind. Componentes usam só tokens --app-* (tokens.css), nunca cores fixas.
- Valores de tema vêm de packages/assets/styles/variables.scss do repositório modrinth/code.
  Copiar valores, não inventar. Ler o LICENSE do pacote antes de copiar arquivos inteiros.
  Nunca usar logo, nome ou ícones da marca Modrinth.

# Ambiente
- Sessões na nuvem (Linux) não têm áudio nem Windows: lá só se escreve e compila código.
  Os testes de áudio da Fase 0 em diante rodam na máquina Windows do usuário.
