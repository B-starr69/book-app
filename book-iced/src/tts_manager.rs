use anyhow::Result;
use kokoro_tts::{KokoroTts, Voice};
use rodio::{OutputStream, OutputStreamBuilder, Sink, buffer::SamplesBuffer};
use std::sync::Arc;

pub struct TtsEngine {
    tts: Arc<KokoroTts>,
    _stream: OutputStream,
    sink: Arc<Sink>,
}

impl TtsEngine {
    pub async fn new(model_path: &str, voices_path: &str) -> anyhow::Result<Self> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        let sink = Sink::connect_new(stream.mixer());

        let tts = KokoroTts::new(model_path, voices_path).await?;

        Ok(Self {
            tts: Arc::new(tts),
            _stream: stream,
            sink: Arc::new(sink),
        })
    }
    pub fn speak(&self, text: String) {
        let tts = Arc::clone(&self.tts);
        let sink = Arc::clone(&self.sink);

        tokio::spawn(async move {
            match tts.synth(&text, Voice::AfHeart(1.0)).await {
                Ok((audio, _took)) => {
                    sink.stop();

                    let source = SamplesBuffer::new(1, 24_000, audio);

                    sink.append(source);
                    sink.play();
                }

                Err(e) => {
                    eprintln!("Kokoro synthesis error: {e}");
                }
            }
        });
    }

    pub fn stop(&self) {
        self.sink.stop();
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn resume(&self) {
        self.sink.play();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_speak() {
        let tts = TtsEngine::new(
            "book-iced/assets/kokoro-v1.0.int8.onnx",
            "book-iced/assets/voices.bin",
        )
        .await
        .unwrap();

        tts.speak("Hello, world!".to_string());

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
