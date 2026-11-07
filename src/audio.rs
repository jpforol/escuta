//! Captura PCM pelo FFmpeg. O áudio existe somente em memória.
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread,
};

pub const RATE: usize = 16_000;
#[derive(Clone, Debug, Deserialize)]
pub struct Source {
    pub name: String,
    pub description: String,
}

pub fn sources() -> Result<Vec<Source>> {
    let output = Command::new("pactl")
        .args(["--format=json", "list", "sources"])
        .output()
        .context("Não foi possível executar pactl")?;
    if !output.status.success() {
        bail!("PipeWire: {}", String::from_utf8_lossy(&output.stderr));
    }
    let mut sources: Vec<Source> = serde_json::from_slice(&output.stdout)?;
    sources.retain(|s| s.name.ends_with(".monitor"));
    Ok(sources)
}
pub fn default_monitor() -> Result<String> {
    let out = Command::new("pactl").arg("get-default-sink").output()?;
    if !out.status.success() {
        bail!("Não foi possível consultar a saída padrão");
    }
    Ok(format!("{}.monitor", String::from_utf8(out.stdout)?.trim()))
}

pub struct Chunk {
    pub samples: Vec<f32>,
    pub offset: f64,
}
// O limite evita crescimento indefinido se a inferência ficar mais lenta que o áudio.
pub struct Capture {
    child: Arc<Mutex<Option<Child>>>,
    pub stop: Arc<AtomicBool>,
}
impl Capture {
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
    pub fn start(&self, source: &str, seconds: Option<u32>) -> Result<Receiver<Result<Chunk>>> {
        anyhow::ensure!(
            sources()?.iter().any(|s| s.name == source),
            "Fonte de áudio indisponível: {source}. Atualize a lista de saídas."
        );
        let mut command = Command::new("ffmpeg");
        command.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-f",
            "pulse",
            "-i",
            source,
        ]);
        if let Some(seconds) = seconds {
            command.args(["-t", &seconds.to_string()]);
        }
        command.args(["-ac", "1", "-ar", "16000", "-f", "f32le", "pipe:1"]);
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut slot = self.child.lock().unwrap();
        if self.stop.load(Ordering::SeqCst) {
            bail!("Captura cancelada");
        }
        let mut child = command.spawn().context("Falha ao iniciar FFmpeg")?;
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        *slot = Some(child);
        drop(slot);
        let (tx, rx) = mpsc::sync_channel(8);
        let child_slot = self.child.clone();
        let stopped = self.stop.clone();
        thread::spawn(move || {
            let errors = thread::spawn(move || {
                let mut message = String::new();
                let _ = stderr.read_to_string(&mut message);
                message
            });
            let result = read_chunks(&mut stdout, &tx);
            // O leitor terminou; coletar o processo também evita processos zumbis.
            let status = child_slot.lock().unwrap().take().map(|mut child| {
                if result.is_err() {
                    let _ = child.kill();
                }
                child.wait()
            });
            let message = errors.join().unwrap_or_default();
            if let Err(error) = result {
                let _ = tx.send(Err(error));
            } else if !stopped.load(Ordering::SeqCst)
                && status.is_some_and(|s| !matches!(s, Ok(s) if s.success()))
            {
                let _ = tx.send(Err(anyhow::anyhow!(
                    "Captura interrompida: {}",
                    message.trim()
                )));
            }
        });
        Ok(rx)
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn emit(tx: &SyncSender<Result<Chunk>>, samples: Vec<f32>, offset: usize) -> Result<()> {
    match tx.try_send(Ok(Chunk {
        samples,
        offset: offset as f64 / RATE as f64,
    })) {
        Ok(()) => Ok(()),
        Err(TrySendError::Full(_)) => bail!(
            "A fila de oito blocos de áudio ficou cheia. Captura interrompida; experimente a GPU ou um modelo menor."
        ),
        Err(TrySendError::Disconnected(_)) => bail!("Transcritor encerrado"),
    }
}

fn read_chunks(input: &mut impl Read, tx: &SyncSender<Result<Chunk>>) -> Result<()> {
    let mut bytes = [0u8; 6400]; // 100 ms, mono, f32 a 16 kHz.
    let mut pending = Vec::new();
    let mut samples = Vec::new();
    let mut offset = 0;
    let mut silence_frames = 0;
    loop {
        let count = input.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        pending.extend_from_slice(&bytes[..count]);
        // Leituras de pipe podem terminar no meio de uma amostra ou de um frame.
        while pending.len() >= bytes.len() {
            let frame: Vec<f32> = pending
                .drain(..bytes.len())
                .collect::<Vec<_>>()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame.len() as f32).sqrt();
            silence_frames = if rms < 0.003 { silence_frames + 1 } else { 0 };
            samples.extend(frame);
            // Preferir pausas da fala; limite de oito segundos para manter o texto chegando.
            if samples.len() >= RATE * 8 || (samples.len() >= RATE * 2 && silence_frames >= 6) {
                let len = samples.len();
                emit(tx, std::mem::take(&mut samples), offset)?;
                offset += len;
                silence_frames = 0;
            }
        }
    }
    samples.extend(
        pending
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b)),
    );
    if !samples.is_empty() {
        emit(tx, samples, offset)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flushes_partial_final_chunk_and_preserves_offsets() {
        let (tx, rx) = mpsc::sync_channel(8);
        let data = vec![0.1f32; RATE * 9 + 17]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        read_chunks(&mut &data[..], &tx).unwrap();
        drop(tx);
        let chunks: Vec<_> = rx.into_iter().map(Result::unwrap).collect();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[1].offset, 8.0);
        assert_eq!(
            chunks.iter().map(|c| c.samples.len()).sum::<usize>(),
            RATE * 9 + 17
        );
    }
    #[test]
    fn stops_when_consumer_cannot_keep_up() {
        let (tx, _rx) = mpsc::sync_channel(1);
        emit(&tx, vec![0.0], 0).unwrap();
        assert!(emit(&tx, vec![0.0], 1).is_err());
    }

    #[test]
    fn preserves_samples_when_reads_end_in_middle_of_float() {
        struct Fragmented(std::io::Cursor<Vec<u8>>);
        impl Read for Fragmented {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                let limit = buffer.len().min(7);
                self.0.read(&mut buffer[..limit])
            }
        }
        let expected = vec![0.15f32; RATE * 2 + 3];
        let bytes = expected
            .iter()
            .copied()
            .flat_map(f32::to_le_bytes)
            .collect();
        let (tx, rx) = mpsc::sync_channel(8);
        read_chunks(&mut Fragmented(std::io::Cursor::new(bytes)), &tx).unwrap();
        drop(tx);
        let actual: Vec<_> = rx.into_iter().flat_map(|c| c.unwrap().samples).collect();
        assert_eq!(actual, expected);
    }
}
