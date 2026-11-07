mod audio;
mod transcribe;
mod ui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{path::PathBuf, process::Command, time::Instant};
use transcribe::{Config, Engine, Event, Session};

#[derive(Parser)]
#[command(version, about = "Escuta — transcrição local para estudar no Firefox")]
struct Args {
    #[arg(long, global = true, help = "Modelo GGML do whisper.cpp")]
    model: Option<PathBuf>,
    #[arg(long, global = true, help = "Desativar a aceleração Vulkan")]
    cpu: bool,
    #[arg(long, hide = true)]
    smoke_test: bool,
    #[command(subcommand)]
    command: Option<Mode>,
}
#[derive(Subcommand)]
enum Mode {
    /// Listar monitores de áudio disponíveis, sem gravar.
    Sources,
    /// Transcrever o som ao vivo no terminal. Use --seconds para um teste limitado.
    Listen {
        #[arg(long)]
        source: Option<String>,
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        seconds: Option<u32>,
        #[arg(long, default_value = "")]
        glossary: String,
    },
    /// Medir a transcrição dos primeiros 30 segundos de um arquivo local.
    Benchmark { file: PathBuf },
}
fn main() -> Result<()> {
    let args = Args::parse();
    let model = args.model.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models/ggml-large-v3-turbo-q5_0.bin")
    });
    let mut config = Config {
        model,
        source: String::new(),
        cpu: args.cpu || !cfg!(feature = "vulkan"),
        glossary: String::new(),
        seconds: None,
    };
    match args.command {
        Some(Mode::Sources) => {
            for source in audio::sources()? {
                println!("{}\t{}", source.name, source.description);
            }
        }
        Some(Mode::Listen {
            source,
            seconds,
            glossary,
        }) => {
            config.source = source.map(Ok).unwrap_or_else(audio::default_monitor)?;
            config.seconds = seconds;
            config.glossary = glossary;
            let session = Session::start(config);
            let mut error = None;
            while let Ok(event) = session.events.recv() {
                match event {
                    Event::Status(text) => eprintln!("{text}"),
                    Event::Text {
                        text,
                        offset,
                        duration,
                        inference,
                        delay,
                    } => {
                        if !text.is_empty() {
                            println!("[{}] {text}", timestamp(offset));
                        }
                        eprintln!(
                            "Bloco {duration:.1}s | inferência {inference:.2}s | atraso após bloco {delay:.2}s"
                        );
                    }
                    Event::Error(text) => {
                        eprintln!("Erro: {text}");
                        error = Some(text);
                    }
                    Event::Finished => break,
                }
            }
            if let Some(error) = error {
                anyhow::bail!(error);
            }
        }
        Some(Mode::Benchmark { file }) => {
            let mut engine = Engine::load(&config)?;
            let decoded = Command::new("ffmpeg")
                .args(["-v", "error", "-nostdin", "-i"])
                .arg(file)
                .args([
                    "-t", "30", "-ac", "1", "-ar", "16000", "-f", "f32le", "pipe:1",
                ])
                .output()
                .context("Não foi possível decodificar o áudio")?;
            anyhow::ensure!(
                decoded.status.success(),
                "{}",
                String::from_utf8_lossy(&decoded.stderr)
            );
            let samples: Vec<f32> = decoded
                .stdout
                .as_chunks::<4>()
                .0
                .iter()
                .map(|s| f32::from_le_bytes(*s))
                .collect();
            anyhow::ensure!(!samples.is_empty(), "Arquivo sem áudio");
            let before = Instant::now();
            println!("{}", engine.transcribe(&samples)?);
            let elapsed = before.elapsed().as_secs_f64();
            let duration = samples.len() as f64 / audio::RATE as f64;
            eprintln!(
                "Áudio: {duration:.2}s | inferência: {elapsed:.2}s | fator de tempo real: {:.3} (menor que 1 acompanha a reprodução)",
                elapsed / duration
            );
        }
        None => ui::launch(config, args.smoke_test).map_err(|e| anyhow::anyhow!("{e}"))?,
    }
    Ok(())
}
pub fn timestamp(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
