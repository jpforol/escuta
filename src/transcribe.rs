//! Inferência local. A interface recebe eventos por um canal, sem bloquear a janela.
use crate::audio::{Capture, Chunk, RATE};
use anyhow::{Context, Result};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::Ordering,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Instant,
};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[derive(Clone)]
pub struct Config {
    pub model: PathBuf,
    pub source: String,
    pub cpu: bool,
    pub glossary: String,
    pub seconds: Option<u32>,
}
#[derive(Debug)]
pub enum Event {
    Status(String),
    Text {
        text: String,
        offset: f64,
        duration: f64,
        inference: f64,
        delay: f64,
    },
    Error(String),
    Finished,
}
pub struct Session {
    pub capture: Arc<Capture>,
    pub events: Receiver<Event>,
}
impl Session {
    pub fn start(config: Config) -> Self {
        let capture = Arc::new(Capture::new());
        let worker_capture = capture.clone();
        let (tx, events) = mpsc::channel();
        thread::spawn(move || {
            if let Err(e) = run(config, &worker_capture, &tx) {
                let _ = tx.send(Event::Error(format!("{e:#}")));
            }
            worker_capture.stop();
            let _ = tx.send(Event::Finished);
        });
        Self { capture, events }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.capture.stop();
    }
}

pub struct Engine {
    _context: WhisperContext,
    state: whisper_rs::WhisperState,
    glossary: String,
}
impl Engine {
    pub fn load(config: &Config) -> Result<Self> {
        anyhow::ensure!(
            config.model.is_file(),
            "Modelo não encontrado: {}. Execute scripts/baixar-modelo.sh.",
            config.model.display()
        );
        let mut params = WhisperContextParameters::default();
        params.use_gpu(!config.cpu);
        let context = WhisperContext::new_with_params(
            config
                .model
                .to_str()
                .context("Caminho do modelo inválido")?,
            params,
        )
        .context("Não foi possível carregar o modelo; tente a opção CPU")?;
        let state = context.create_state()?;
        Ok(Self {
            _context: context,
            state,
            glossary: config.glossary.clone(),
        })
    }
    pub fn transcribe(&mut self, audio: &[f32]) -> Result<String> {
        let rms = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len().max(1) as f32).sqrt();
        if rms < 0.001 {
            return Ok(String::new());
        }
        let mut padded = audio.to_vec();
        padded.resize(padded.len().max(RATE), 0.0);
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(6);
        params.set_language(Some("pt"));
        params.set_translate(false);
        params.set_no_context(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        if !self.glossary.trim().is_empty() {
            params.set_initial_prompt(&self.glossary);
        }
        self.state.full(params, &padded)?;
        let mut text = String::new();
        for segment in self.state.as_iter() {
            text.push_str(segment.to_str()?);
        }
        Ok(text.trim().to_owned())
    }
}
fn run(config: Config, capture: &Capture, tx: &Sender<Event>) -> Result<()> {
    let _ = tx.send(Event::Status("Carregando modelo local…".into()));
    let mut engine = Engine::load(&config)?;
    if capture.stop.load(Ordering::SeqCst) {
        return Ok(());
    }
    let rx = capture.start(&config.source, config.seconds)?;
    let started = Instant::now();
    let _ = tx.send(Event::Status(
        "Capturando áudio • processamento local".into(),
    ));
    for chunk in rx {
        let Chunk { samples, offset } = chunk?;
        let duration = samples.len() as f64 / RATE as f64;
        let before = Instant::now();
        let text = engine.transcribe(&samples)?;
        let inference = before.elapsed().as_secs_f64();
        let delay = (started.elapsed().as_secs_f64() - offset - duration).max(0.0);
        if tx
            .send(Event::Text {
                text,
                offset,
                duration,
                inference,
                delay,
            })
            .is_err()
        {
            break;
        }
    }
    Ok(())
}
