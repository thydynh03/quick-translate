//! On-device translation with Opus-MT (Marian) models in ONNX format, run by
//! ONNX Runtime. One model translates one direction; other pairs are routed
//! through English when both halves are installed.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use anyhow::{anyhow, bail, Context};
use futures_util::StreamExt;
use ort::session::Session;
use ort::value::{Tensor, TensorRef};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokenizers::Tokenizer;

use super::{lang, Translation};
use crate::settings::data_dir;

/// Directions that have a converted model in the `Xenova/opus-mt-*` repos.
const PAIRS: &[(&str, &str)] = &[
    ("en", "vi"),
    ("vi", "en"),
    ("ja", "en"),
    ("ko", "en"),
    ("zh", "en"),
    ("en", "zh"),
    ("fr", "en"),
    ("en", "fr"),
    ("de", "en"),
    ("en", "de"),
    ("es", "en"),
    ("en", "es"),
    ("ru", "en"),
    ("en", "ru"),
];

/// File in the repo, and its share of the download (for the progress bar).
const FILES: &[(&str, f64)] = &[
    ("config.json", 0.0),
    ("tokenizer.json", 0.05),
    ("onnx/encoder_model_quantized.onnx", 0.44),
    ("onnx/decoder_model_quantized.onnx", 0.51),
];
const READY_MARKER: &str = "ready";
const APPROX_SIZE_MB: u32 = 107;
const MAX_INPUT_TOKENS: usize = 256;

pub fn models_dir(app: &AppHandle) -> PathBuf {
    data_dir(app).join("models")
}

fn model_dir(root: &Path, from: &str, to: &str) -> PathBuf {
    root.join(format!("{from}-{to}"))
}

fn installed(root: &Path, from: &str, to: &str) -> bool {
    model_dir(root, from, to).join(READY_MARKER).exists()
}

// ------------------------------------------------------------------ models

struct Model {
    encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
    start_token: i64,
    eos_token: i64,
    pad_token: i64,
}

static LOADED: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<Model>>>>> = OnceLock::new();

fn loaded() -> &'static Mutex<HashMap<PathBuf, Arc<Mutex<Model>>>> {
    LOADED.get_or_init(|| Mutex::new(HashMap::new()))
}

fn load(dir: &Path) -> anyhow::Result<Arc<Mutex<Model>>> {
    if let Some(model) = loaded().lock().get(dir) {
        return Ok(model.clone());
    }
    let config: Value = serde_json::from_str(&fs::read_to_string(dir.join("config.json"))?)?;
    let id = |key: &str| config.get(key).and_then(Value::as_i64).ok_or_else(|| anyhow!("config.json thiếu {key}"));
    let session = |file: &str| -> anyhow::Result<Session> {
        Session::builder()
            .map_err(|e| anyhow!("{e}"))?
            .commit_from_file(dir.join(file))
            .map_err(|e| anyhow!("Không nạp được {file}: {e}"))
    };
    let model = Model {
        encoder: session("encoder_model_quantized.onnx")?,
        decoder: session("decoder_model_quantized.onnx")?,
        tokenizer: load_tokenizer(&dir.join("tokenizer.json"))?,
        start_token: id("decoder_start_token_id")?,
        eos_token: id("eos_token_id")?,
        pad_token: id("pad_token_id")?,
    };
    let model = Arc::new(Mutex::new(model));
    loaded().lock().insert(dir.to_path_buf(), model.clone());
    Ok(model)
}

/// The converted repos ship a SentencePiece "Precompiled" normalizer with an
/// empty character map, which the tokenizers crate rejects. NFKC is what that
/// map encodes for these models, so use it instead.
fn load_tokenizer(path: &Path) -> anyhow::Result<Tokenizer> {
    let mut json: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    let empty_precompiled = json
        .get("normalizer")
        .is_some_and(|n| n.get("type").and_then(Value::as_str) == Some("Precompiled") && n["precompiled_charsmap"].is_null());
    if empty_precompiled {
        json["normalizer"] = serde_json::json!({ "type": "NFKC" });
    }
    Tokenizer::from_bytes(serde_json::to_vec(&json)?).map_err(|e| anyhow!("tokenizer: {e}"))
}

impl Model {
    /// Greedy decoding of one sentence.
    fn translate_sentence(&mut self, sentence: &str) -> anyhow::Result<String> {
        let encoding = self.tokenizer.encode(sentence, true).map_err(|e| anyhow!("tokenizer: {e}"))?;
        let mut input: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        if input.len() > MAX_INPUT_TOKENS {
            input.truncate(MAX_INPUT_TOKENS - 1);
            input.push(self.eos_token);
        }
        if input.last() != Some(&self.eos_token) {
            input.push(self.eos_token);
        }
        let n = input.len();
        let mask = vec![1i64; n];

        let (hidden, width) = {
            let outputs = self
                .encoder
                .run(ort::inputs![
                    "input_ids" => Tensor::from_array(([1usize, n], input))?,
                    "attention_mask" => Tensor::from_array(([1usize, n], mask.clone()))?,
                ])
                .map_err(|e| anyhow!("encoder: {e}"))?;
            let (shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(|e| anyhow!("{e}"))?;
            (data.to_vec(), shape[2] as usize)
        };

        let max_new = (n * 2 + 16).min(400);
        let mut output: Vec<i64> = vec![self.start_token];
        for _ in 0..max_new {
            let t = output.len();
            let outputs = self
                .decoder
                .run(ort::inputs![
                    "input_ids" => Tensor::from_array(([1usize, t], output.clone()))?,
                    "encoder_attention_mask" => TensorRef::from_array_view(([1usize, n], mask.as_slice()))?,
                    "encoder_hidden_states" => TensorRef::from_array_view(([1usize, n, width], hidden.as_slice()))?,
                ])
                .map_err(|e| anyhow!("decoder: {e}"))?;
            let (shape, logits) = outputs[0].try_extract_tensor::<f32>().map_err(|e| anyhow!("{e}"))?;
            let vocab = shape[2] as usize;
            let last = &logits[(t - 1) * vocab..t * vocab];
            // An empty translation is never right: no end-of-sentence first.
            let banned: &[usize] = if t == 1 { &[self.pad_token as usize, self.eos_token as usize] } else { &[self.pad_token as usize] };
            let next = argmax(last, banned) as i64;
            if next == self.eos_token {
                break;
            }
            // The same token three times in a row is the model stuck in a loop:
            // keep one and stop instead of filling the answer with it.
            if output.len() >= 3 && output[output.len() - 2..] == [next, next] {
                output.pop();
                break;
            }
            output.push(next);
        }

        let ids: Vec<u32> = output[1..].iter().map(|&id| id as u32).collect();
        let text = self.tokenizer.decode(&ids, true).map_err(|e| anyhow!("tokenizer: {e}"))?;
        Ok(text.trim().to_string())
    }
}

/// Index of the largest value outside anned.
fn argmax(values: &[f32], banned: &[usize]) -> usize {
    let mut best = 0usize;
    let mut best_value = f32::NEG_INFINITY;
    for (index, &value) in values.iter().enumerate() {
        if value > best_value && !banned.contains(&index) {
            best = index;
            best_value = value;
        }
    }
    best
}

/// Splits text into sentences, keeping each sentence's end punctuation.
/// The models were trained on single sentences.
pub fn sentences(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        current.push(c);
        let ends = matches!(c, '.' | '!' | '?' | '。' | '！' | '？');
        let boundary = match chars.peek() {
            None => true,
            Some(next) => next.is_whitespace() || matches!(c, '。' | '！' | '？'),
        };
        if ends && boundary && current.trim().chars().count() > 1 {
            result.push(current.trim().to_string());
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }
    result
}

fn translate_with(dir: &Path, text: &str) -> anyhow::Result<String> {
    let model = load(dir)?;
    let mut model = model.lock();
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let parts: Result<Vec<String>, _> =
            sentences(line).iter().map(|sentence| model.translate_sentence(sentence)).collect();
        lines.push(parts?.join(" "));
    }
    Ok(lines.join("\n"))
}

/// Model folders to run in order for a direction: direct, or through English.
pub fn route(root: &Path, from: &str, to: &str) -> Option<Vec<PathBuf>> {
    if installed(root, from, to) {
        Some(vec![model_dir(root, from, to)])
    } else if from != "en" && to != "en" && installed(root, from, "en") && installed(root, "en", to) {
        Some(vec![model_dir(root, from, "en"), model_dir(root, "en", to)])
    } else {
        None
    }
}

/// Blocking; call from a worker thread.
pub fn translate(root: &Path, text: &str, from: &str, to: &str) -> anyhow::Result<Translation> {
    // Without a network detector, unmarked Latin text is taken as English.
    let from = if from == "auto" { lang::guess(text).unwrap_or_else(|| "en".into()) } else { from.to_string() };
    if from == to {
        bail!("Văn bản đã ở ngôn ngữ đích");
    }
    // Bare words: the bundled word list beats the sentence model.
    if let Some(found) = super::wordbook::find(text, &from, to) {
        if let Some(first) = found.senses.first().and_then(|sense| sense.terms.first()) {
            return Ok(Translation {
                text: first.clone(),
                from,
                to: to.to_string(),
                engine: "offline".into(),
                phonetic: found.phonetic,
                note: found.stem.map(|stem| format!("Nghĩa của từ gốc “{stem}”.")),
                dict: found.senses,
                ..Default::default()
            });
        }
    }
    // The sentence models turn a lone word into nonsense ("institution" came
    // out as one word repeated twenty times). Failing here lets an online
    // engine answer when there is a network.
    if lang::is_single_word(text) && [from.as_str(), to].iter().all(|code| ["en", "vi"].contains(code)) {
        bail!("Từ này không có trong từ điển offline; cần mạng để tra");
    }
    let steps = route(root, &from, to)
        .ok_or_else(|| anyhow!("Chưa tải model offline {} → {}", from.to_uppercase(), to.to_uppercase()))?;
    let mut current = text.to_string();
    for dir in &steps {
        current = translate_with(dir, &current).with_context(|| format!("model {}", dir.display()))?;
    }
    Ok(Translation {
        text: current,
        from,
        to: to.to_string(),
        engine: "offline".into(),
        note: (steps.len() > 1).then(|| "Dịch qua trung gian tiếng Anh.".to_string()),
        ..Default::default()
    })
}

// ---------------------------------------------------------------- download

#[derive(Serialize, Clone)]
pub struct ModelInfo {
    id: String,
    from: String,
    to: String,
    size_mb: u32,
    installed: bool,
}

#[derive(Serialize, Clone)]
struct Progress {
    id: String,
    done: u64,
    total: u64,
    error: Option<String>,
    finished: bool,
}

#[tauri::command]
pub fn offline_models(app: AppHandle) -> Vec<ModelInfo> {
    let root = models_dir(&app);
    PAIRS
        .iter()
        .map(|(from, to)| ModelInfo {
            id: format!("{from}-{to}"),
            from: from.to_string(),
            to: to.to_string(),
            size_mb: APPROX_SIZE_MB,
            installed: installed(&root, from, to),
        })
        .chain(std::iter::once(ModelInfo {
            id: super::ocr_vi::ID.to_string(),
            from: "ocr".to_string(),
            to: "vi".to_string(),
            size_mb: super::ocr_vi::APPROX_SIZE_MB,
            installed: super::ocr_vi::installed(&root),
        }))
        .collect()
}

fn pair_of(id: &str) -> Result<(&'static str, &'static str), String> {
    if id == super::ocr_vi::ID {
        return Ok(("ocr", "vi"));
    }
    PAIRS
        .iter()
        .copied()
        .find(|(from, to)| format!("{from}-{to}") == id)
        .ok_or_else(|| format!("Không có model {id}"))
}

/// Downloads one model's files into `dir`, reporting progress in permille.
pub async fn download(dir: &Path, from: &str, to: &str, progress: impl FnMut(u64)) -> anyhow::Result<()> {
    download_files(dir, &format!("https://huggingface.co/Xenova/opus-mt-{from}-{to}/resolve/main"), FILES, progress).await
}

/// Downloads `files` (name, share of the total size) from `base` into `dir`.
pub async fn download_files(
    dir: &Path,
    base_url: &str,
    files: &[(&str, f64)],
    mut progress: impl FnMut(u64),
) -> anyhow::Result<()> {
    fs::create_dir_all(dir)?;
    let client = reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(20)).build()?;
    let mut base = 0.0;
    for (file, share) in files {
        let url = format!("{base_url}/{file}");
        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            bail!("Tải {file} lỗi {}", response.status());
        }
        let length = response.content_length().unwrap_or(0);
        let name = file.rsplit('/').next().unwrap_or(file);
        let partial = dir.join(format!("{name}.part"));
        let mut out = fs::File::create(&partial)?;
        let mut received = 0u64;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            std::io::Write::write_all(&mut out, &chunk)?;
            received += chunk.len() as u64;
            if length > 0 {
                progress(((base + share * received as f64 / length as f64) * 1000.0) as u64);
            }
        }
        drop(out);
        if length > 0 && received != length {
            bail!("Tải {file} bị ngắt giữa chừng");
        }
        fs::rename(&partial, dir.join(name))?;
        base += share;
    }
    fs::write(dir.join(READY_MARKER), "")?;
    Ok(())
}

#[tauri::command]
pub fn offline_download(app: AppHandle, id: String) -> Result<(), String> {
    let (from, to) = pair_of(&id)?;
    let dir = model_dir(&models_dir(&app), from, to);
    tauri::async_runtime::spawn(async move {
        let emit = |done: u64, error: Option<String>, finished: bool| {
            let _ = app.emit("offline-progress", Progress { id: id.clone(), done, total: 1000, error, finished });
        };
        let mut last = 0;
        let report = |permille| {
            if permille != last {
                last = permille;
                emit(permille, None, false);
            }
        };
        let result = if from == "ocr" {
            download_files(&dir, super::ocr_vi::BASE_URL, super::ocr_vi::FILES, report).await
        } else {
            download(&dir, from, to, report).await
        };
        match result {
            Ok(()) => emit(1000, None, true),
            Err(error) => {
                crate::log(format!("offline download {id}: {error:#}"));
                emit(0, Some(error.to_string()), true);
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn offline_remove(app: AppHandle, id: String) -> Result<(), String> {
    let (from, to) = pair_of(&id)?;
    let dir = model_dir(&models_dir(&app), from, to);
    loaded().lock().remove(&dir);
    super::ocr_vi::unload();
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_sentences() {
        assert_eq!(sentences("Hello there. How are you? Fine!"), ["Hello there.", "How are you?", "Fine!"]);
        assert_eq!(sentences("No punctuation here"), ["No punctuation here"]);
        assert_eq!(sentences("Version 3.5 is out. Try it."), ["Version 3.5 is out.", "Try it."]);
        assert_eq!(sentences("今日は晴れ。散歩しよう。"), ["今日は晴れ。", "散歩しよう。"]);
    }

    #[test]
    fn argmax_skips_banned_tokens() {
        assert_eq!(argmax(&[0.1, 0.9, 5.0], &[2]), 1);
        assert_eq!(argmax(&[0.1, 0.9, 5.0], &[0]), 2);
        assert_eq!(argmax(&[0.1, 0.9, 5.0], &[1, 2]), 0);
    }

    #[test]
    fn routes_direct_then_through_english() {
        let root = std::env::temp_dir().join(format!("qt-route-{}", std::process::id()));
        let mark = |from: &str, to: &str| {
            let dir = model_dir(&root, from, to);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(READY_MARKER), "").unwrap();
        };
        assert!(route(&root, "ja", "vi").is_none());
        mark("ja", "en");
        assert!(route(&root, "ja", "vi").is_none());
        mark("en", "vi");
        assert_eq!(route(&root, "ja", "vi").unwrap().len(), 2);
        assert_eq!(route(&root, "en", "vi").unwrap().len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    /// Downloads the real en→vi and vi→en models (about 210 MB) and translates.
    /// Run with: cargo test offline_end_to_end -- --ignored --nocapture
    #[test]
    #[ignore]
    fn offline_end_to_end() {
        let root = std::env::temp_dir().join("qt-offline-test");
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        for (from, to) in [("en", "vi"), ("vi", "en")] {
            if !installed(&root, from, to) {
                runtime.block_on(download(&model_dir(&root, from, to), from, to, |_| {})).unwrap();
            }
        }
        let cases = [
            ("The weather is beautiful today. Let's go for a walk in the park.", "auto", "vi"),
            ("Tôi đang học lập trình để xây dựng phần mềm dịch.", "vi", "en"),
            ("book", "en", "vi"),
        ];
        // A word outside the bundled list is refused, so an online engine can take over.
        assert!(translate(&root, "qzxv", "en", "vi").is_err());
        for (text, from, to) in cases {
            let started = std::time::Instant::now();
            let result = translate(&root, text, from, to).unwrap();
            println!("[{} ms] {} -> {}", started.elapsed().as_millis(), text, result.text);
            assert!(!result.text.trim().is_empty());
        }
    }
}
