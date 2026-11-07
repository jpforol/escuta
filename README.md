# Escuta

Aplicativo Linux para transcrever o áudio que você ouve enquanto estuda no Firefox.
Interface e controle em Rust, inferência local com `whisper-rs`/`whisper.cpp` e captura
via FFmpeg/PipeWire. Não depende de Python, conta da Hotmart ou acesso ao arquivo do vídeo.

## Usar nesta máquina

```bash
./iniciar.sh
```

Selecione o monitor da saída em que o Firefox toca (nesta máquina, Mesa de Som),
abra a aula e clique em **Iniciar**. Use **Parar** para encerrar a captura e terminar
os blocos pendentes.
O botão **Reset** interrompe a captura, descarta o texto da janela e limpa as
estatísticas. Clique em **Iniciar** para uma nova captura com horários a partir
de zero. As configurações e os arquivos já exportados são mantidos.

A captura começa somente depois que o modelo está carregado;
trechos anteriores não são recuperados. O texto aparece em blocos de até oito segundos,
mais o tempo de processamento; não são legendas instantâneas palavra a palavra.

Em “Exportar para estudo e refinamento”, salve o original em Markdown ou prepare
uma cópia com instruções para revisão pelo seu modelo. **Não há integração automática
com um modelo de refinamento nesta versão.** Nenhum áudio é enviado a serviços externos.
Horários são relativos ao início de cada captura, não à posição do player da aula.

O texto fica na memória até exportar. Fechar o aplicativo descarta texto não salvo.
Uma nova captura preserva o texto anterior, separando os relógios de cada sessão.
Capturar o monitor da saída inclui notificações e outros aplicativos que usem essa
saída. O canal de mídia pode ser selecionado se você já tiver roteado o Firefox para ele;
o aplicativo não muda seu roteamento de áudio.

## Compilar

Requer Rust/Cargo, compiladores C/C++, CMake, libclang, FFmpeg e pactl. Para a versão
Vulkan, requer também o loader/driver Vulkan e `glslc`. O script abaixo baixa somente
os cabeçalhos de desenvolvimento para `.deps`, sem modificar pacotes do sistema.

```bash
./scripts/preparar-vulkan.sh
./scripts/baixar-modelo.sh
cargo build --release --locked -j 6
```

O modelo padrão é Whisper Large v3 Turbo multilíngue quantizado Q5_0 (~547 MiB).
O Small Q5_1 (~182 MiB) também está disponível nesta máquina como alternativa leve;
em outra instalação, baixe-o com `./scripts/baixar-modelo.sh small`. Os pesos usados
por `faster-whisper` têm outro formato; por isso há um download específico GGML.
Os downloads ocorrem na preparação. O aplicativo não baixa modelos ao iniciar.

Para compilar sem Vulkan, use `cargo build --release --locked --no-default-features`.
Para desativar a GPU em tempo de execução, marque CPU na interface ou use `--cpu`.

## Terminal e verificação

```bash
./iniciar.sh sources
./iniciar.sh listen --seconds 20
./iniciar.sh --cpu --model models/ggml-small-q5_1.bin listen --seconds 20
./iniciar.sh benchmark caminho/para/amostra.wav
cargo test --release --locked -j 6
cargo clippy --release --locked -- -D warnings
./scripts/testar-fluxo.sh
```

`benchmark` usa no máximo os primeiros 30 segundos de um arquivo que você já possui;
ele não baixa conteúdo. O fator de tempo real é tempo de inferência / duração do áudio.
Menor que 1 indica capacidade de acompanhar aquela amostra, não garantia para toda aula.

O teste opcional `testar-fluxo.sh` requer `espeak-ng`, `paplay` e `rg`. Ele cria
um canal temporário de áudio, injeta uma frase sintética e verifica se a captura
gera texto. O canal é removido ao terminar; a saída padrão não é alterada.

## Aprender Rust neste projeto

- `src/main.rs`: entrada do aplicativo, argumentos com `clap`, `match` e propagação
  de erros com `Result` e `?`.
- `src/audio.rs`: processos com `Command`, leitura de bytes, conversão para `f32`,
  `Vec`, threads, canais limitados e encerramento por `Drop`.
- `src/transcribe.rs`: `struct` para configuração, `enum Event` para os eventos e
  trabalho em outra thread com `move`. A janela não executa a inferência.
- `src/ui.rs`: implementação do trait `eframe::App`, estado da interface e exportação.

Veja também [o roteiro de leitura com exemplos de sintaxe](docs/roteiro-rust.md).

`Arc` compartilha a captura entre threads. `Mutex` protege o processo do FFmpeg;
`AtomicBool` comunica a parada. O canal de áudio tem capacidade limitada: quando
o processamento não acompanha a captura, o aplicativo interrompe com erro explícito
em vez de consumir memória sem limite ou descartar trechos silenciosamente.

## Limites da primeira versão

Há detecção simples de silêncio por energia, não um detector neural de voz. Música,
ruído e palavras cortadas no limite dos blocos podem causar erros de transcrição.
O refinamento deve preservar o original, não inventar conteúdo ou preencher lacunas.
Não foi implementada recuperação automática após falha, persistência automática ou
diarização de falantes. A compatibilidade específica com uma aula deve ser verificada
durante a reprodução; o aplicativo captura uma saída de áudio do sistema.

Fontes: [whisper-rs](https://docs.rs/whisper-rs/0.16.0/whisper_rs/),
[whisper.cpp](https://github.com/ggml-org/whisper.cpp),
[egui/eframe](https://docs.rs/eframe/0.37.0/eframe/),
[modelo GGML](https://huggingface.co/ggerganov/whisper.cpp).
