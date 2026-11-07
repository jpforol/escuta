# Um roteiro curto para aprender com o Escuta

Comece pelo `src/main.rs` e acompanhe o caminho de `Mode::Listen`.

```rust
let session = Session::start(config);
while let Ok(event) = session.events.recv() {
    match event {
        Event::Status(text) => eprintln!("{text}"),
        Event::Finished => break,
        _ => {} // Os outros eventos são tratados no código completo.
    }
}
```

- `let` cria uma associação imutável por padrão; `let mut` permite alterá-la.
- `::` acessa um item de um tipo ou módulo, como `Session::start`.
- `Result<T, E>` representa sucesso (`Ok`) ou erro (`Err`). `?` propaga o erro
  ao chamador quando a função também retorna um resultado compatível.
- `enum Event` descreve alternativas que podem carregar dados diferentes.
  `match` examina a alternativa e extrai seus dados.
- `while let` repete enquanto um padrão continua correspondendo: aqui,
  enquanto há eventos chegando pelo canal.

Depois leia `Session::start` em `src/transcribe.rs`. A palavra `move` na closure
entrega a propriedade das variáveis capturadas para a thread que vai usá-las.
Isso evita que a thread dependa de variáveis locais que já deixaram de existir.

Em `src/audio.rs`, observe `Drop`: quando uma captura deixa de ser usada,
ela pede o encerramento do processo. O uso de `Arc` permite compartilhar a
captura; o uso de `Mutex` impede acesso simultâneo desprotegido ao processo.

Exercícios pequenos e verificáveis:

1. Acrescente um argumento de idioma ao CLI e passe-o a `set_language`.
2. Torne o limite de oito segundos configurável entre quatro e quinze segundos.
3. Acrescente exportação `.txt`, mantendo a transcrição original intacta.

Para cada mudança, rode `cargo fmt`, `cargo test --release` e experimente uma
amostra curta. Os testes em `audio.rs` mostram por que uma leitura do pipe não
precisa terminar no limite de uma amostra de áudio.
