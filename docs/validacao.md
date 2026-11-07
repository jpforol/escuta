# Validação local — 6 de novembro de 2026

Máquina: CachyOS, Ryzen 5 5600X, 32 GB de RAM, Radeon Navi 23 de 8 GB.
O backend Vulkan foi identificado e usado pelo `whisper.cpp`.

## Resultado observado

| Amostra sintética em português (12,50 s) | Inferência | Fator de tempo real |
| --- | --- | --- |
| Small Q5_1, CPU | 2,55 s | 0,204 |
| Small Q5_1, Vulkan | 0,90 s | 0,072 |
| Large v3 Turbo Q5_0, Vulkan | 0,92 s | 0,073 |

Tempos de inferência excluem carregamento inicial do modelo. São medições únicas,
não um benchmark estatístico. O Turbo errou menos palavras nessa amostra, mas
ainda confundiu “Rust” com “custo”. Isso não mede a precisão em videoaulas reais.

No teste com áudio injetado num canal PipeWire temporário, a frase foi:

> Esta é uma aula de programação. Vamos estudar variáveis e funções.

O Turbo reproduziu a frase corretamente. Um bloco de 6,4 s levou aproximadamente
0,77 s para processar, com 0,86 s de atraso após o fim do bloco. O canal de teste
foi removido; a saída padrão e o roteamento do Firefox foram preservados.

A captura na saída real do usuário foi testada durante 20 s e recebeu silêncio.
Ainda falta validar precisão com fala da aula em reprodução.

## Verificações

- Compilação release concluída; Clippy sem avisos.
- Quatro testes passaram: preservação das amostras em leituras fragmentadas,
  horários e bloco final, limite da fila e exportação sem alterar o original.
- Janela nativa aberta e inspecionada por screenshot do próprio aplicativo.
- Fonte de áudio inexistente rejeitada com erro explícito.
- Scripts de preparação e inicialização passaram na análise de sintaxe do Bash.

Os arquivos gerados pelos testes estão em `test-artifacts/` e são ignorados pelo Git.
