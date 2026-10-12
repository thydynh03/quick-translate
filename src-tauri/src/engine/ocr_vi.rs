//! Vietnamese text recognition with VietOCR (VGG encoder + transformer decoder)
//! exported to ONNX. It reads one cropped piece of a text line at a time; the
//! platform's own OCR finds where the lines and words are.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{anyhow, bail};
use ort::session::Session;
use ort::value::{Tensor, TensorRef};
use parking_lot::Mutex;

use crate::platform::Rect;

pub const ID: &str = "ocr-vi";
pub const APPROX_SIZE_MB: u32 = 147;
pub const BASE_URL: &str = "https://huggingface.co/vemines/vietocr-onnx/resolve/main";
pub const FILES: &[(&str, f64)] = &[
    ("vocab.json", 0.0),
    ("vgg_encoder.onnx", 0.01),
    ("vgg_encoder.onnx.data", 0.73),
    ("vgg_decoder.onnx", 0.26),
];

const HEIGHT: usize = 32;
const MIN_WIDTH: usize = 32;
const MAX_WIDTH: usize = 512;
/// A piece wider than this many line heights would be squeezed past reading.
const MAX_ASPECT: f32 = 13.0;
const MAX_CHARS: usize = 160;
const SOS: i64 = 1;
const EOS: i64 = 2;
/// Tokens 0..4 are padding, start, end and mask; characters follow.
const FIRST_CHAR: usize = 4;

struct Model {
    encoder: Session,
    decoder: Session,
    chars: Vec<char>,
}

fn slot() -> &'static Mutex<Option<(PathBuf, Model)>> {
    static SLOT: OnceLock<Mutex<Option<(PathBuf, Model)>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

pub fn dir(root: &Path) -> PathBuf {
    root.join(ID)
}

pub fn installed(root: &Path) -> bool {
    dir(root).join("ready").exists()
}

/// Frees the loaded model so its files can be deleted.
pub fn unload() {
    *slot().lock() = None;
}

fn load(dir: &Path) -> anyhow::Result<Model> {
    let session = |file: &str| -> anyhow::Result<Session> {
        Session::builder()
            .map_err(|e| anyhow!("{e}"))?
            .commit_from_file(dir.join(file))
            .map_err(|e| anyhow!("Không nạp được {file}: {e}"))
    };
    let vocab: String = serde_json::from_str(&std::fs::read_to_string(dir.join("vocab.json"))?)?;
    Ok(Model { encoder: session("vgg_encoder.onnx")?, decoder: session("vgg_decoder.onnx")?, chars: vocab.chars().collect() })
}

/// Scales an RGB image to the model's input height with bilinear sampling and
/// returns it as planar floats in 0..1 plus the new width.
fn prepare(rgb: &[u8], width: usize, height: usize) -> (Vec<f32>, usize) {
    let scaled = (HEIGHT * width).div_ceil(height);
    let new_width = (scaled.div_ceil(10) * 10).clamp(MIN_WIDTH, MAX_WIDTH);
    let mut out = vec![0f32; 3 * HEIGHT * new_width];
    let (sx, sy) = (width as f32 / new_width as f32, height as f32 / HEIGHT as f32);
    for y in 0..HEIGHT {
        let fy = ((y as f32 + 0.5) * sy - 0.5).clamp(0.0, (height - 1) as f32);
        let (y0, ty) = (fy as usize, fy.fract());
        let y1 = (y0 + 1).min(height - 1);
        for x in 0..new_width {
            let fx = ((x as f32 + 0.5) * sx - 0.5).clamp(0.0, (width - 1) as f32);
            let (x0, tx) = (fx as usize, fx.fract());
            let x1 = (x0 + 1).min(width - 1);
            for channel in 0..3 {
                let at = |px: usize, py: usize| rgb[(py * width + px) * 3 + channel] as f32;
                let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
                let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
                out[channel * HEIGHT * new_width + y * new_width + x] = (top * (1.0 - ty) + bottom * ty) / 255.0;
            }
        }
    }
    (out, new_width)
}

fn owned(value: &ort::value::DynValue) -> anyhow::Result<(Vec<usize>, Vec<f32>)> {
    let (shape, data) = value.try_extract_tensor::<f32>().map_err(|e| anyhow!("{e}"))?;
    Ok((shape.iter().map(|d| *d as usize).collect(), data.to_vec()))
}

impl Model {
    /// Reads the text in one RGB image of a single line (or part of one).
    fn read(&mut self, rgb: &[u8], width: usize, height: usize) -> anyhow::Result<String> {
        let (input, new_width) = prepare(rgb, width, height);
        let image_name = self.encoder.inputs()[0].name().to_string();
        // Features per image column: [time, 1, width].
        let memory = {
            let outputs = self
                .encoder
                .run(ort::inputs![image_name.as_str() => Tensor::from_array(([1usize, 3, HEIGHT, new_width], input))?])
                .map_err(|e| anyhow!("{e}"))?;
            owned(&outputs[0])?
        };

        let classes = self.chars.len() + FIRST_CHAR;
        let mut text = String::new();
        let mut tokens = vec![SOS];
        while tokens.len() <= MAX_CHARS {
            // The decoder takes every token so far as [length, 1] and scores each position.
            let outputs = self
                .decoder
                .run(ort::inputs![
                    "tgt" => Tensor::from_array(([tokens.len(), 1usize], tokens.clone()))?,
                    "memory" => TensorRef::from_array_view((memory.0.clone(), memory.1.as_slice()))?,
                ])
                .map_err(|e| anyhow!("{e}"))?;
            let (_, scores) = owned(&outputs[0])?;
            if scores.len() < classes {
                bail!("model OCR tiếng Việt có đầu ra không như mong đợi");
            }
            let last = &scores[scores.len() - classes..];
            let token = last
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(index, _)| index as i64)
                .unwrap_or(EOS);
            if token == EOS {
                break;
            }
            tokens.push(token);
            if let Some(character) = (token as usize).checked_sub(FIRST_CHAR).and_then(|i| self.chars.get(i)) {
                text.push(*character);
            }
        }
        Ok(text.trim().to_string())
    }
}

/// Groups the words of a line into pieces narrow enough for the recogniser.
pub fn pieces(words: &[Rect]) -> Vec<Rect> {
    let mut pieces: Vec<Rect> = Vec::new();
    for word in words {
        match pieces.last_mut() {
            Some(piece) => {
                let joined = piece.union(word);
                let aspect = (joined.right - joined.left) as f32 / (joined.bottom - joined.top).max(1) as f32;
                if aspect <= MAX_ASPECT {
                    *piece = joined;
                } else {
                    pieces.push(*word);
                }
            }
            None => pieces.push(*word),
        }
    }
    pieces
}

/// Reads one text line from a BGRA screenshot, given the boxes of its words.
pub fn read_line(root: &Path, bgra: &[u8], width: i32, height: i32, words: &[Rect]) -> anyhow::Result<String> {
    let dir = dir(root);
    let mut slot = slot().lock();
    if slot.as_ref().is_none_or(|(loaded, _)| *loaded != dir) {
        *slot = Some((dir.clone(), load(&dir)?));
    }
    let model = &mut slot.as_mut().expect("model was just loaded").1;

    let mut parts = Vec::new();
    for piece in pieces(words) {
        // Tone marks sit above and below what a non-Vietnamese OCR measures.
        let margin = ((piece.bottom - piece.top) as f32 * 0.3).ceil() as i32;
        let (left, right) = ((piece.left - 3).max(0), (piece.right + 3).min(width));
        let (top, bottom) = ((piece.top - margin).max(0), (piece.bottom + margin).min(height));
        if right - left < 2 || bottom - top < 2 {
            continue;
        }
        let (w, h) = ((right - left) as usize, (bottom - top) as usize);
        let mut rgb = Vec::with_capacity(w * h * 3);
        for y in top..bottom {
            let row = (y * width + left) as usize * 4;
            for pixel in bgra[row..row + w * 4].chunks_exact(4) {
                rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
            }
        }
        let text = model.read(&rgb, w, h)?;
        if !text.is_empty() {
            parts.push(text);
        }
    }
    Ok(parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(left: i32, right: i32) -> Rect {
        Rect { left, top: 0, right, bottom: 20 }
    }

    #[test]
    fn short_line_stays_in_one_piece() {
        assert_eq!(pieces(&[word(0, 60), word(70, 150)]).len(), 1);
    }

    #[test]
    fn long_line_is_split_between_words() {
        let words: Vec<Rect> = (0..12).map(|i| word(i * 100, i * 100 + 90)).collect();
        let pieces = pieces(&words);
        assert!(pieces.len() > 1);
        assert!(pieces.iter().all(|p| (p.right - p.left) as f32 / 20.0 <= MAX_ASPECT));
    }

    #[test]
    fn scales_to_model_height() {
        let (data, width) = prepare(&vec![255u8; 100 * 20 * 3], 100, 20);
        assert_eq!(width, 160);
        assert_eq!(data.len(), 3 * HEIGHT * 160);
        assert!(data.iter().all(|v| (*v - 1.0).abs() < 1e-6));
    }

    /// Needs the model files: QT_OCR_VI_ROOT=<dir containing ocr-vi/>, and
    /// QT_OCR_VI_SAMPLE=<raw BGRA file> with QT_OCR_VI_SIZE=<width>x<height>.
    #[test]
    #[ignore]
    fn reads_a_vietnamese_line() {
        let root = PathBuf::from(std::env::var("QT_OCR_VI_ROOT").unwrap());
        let bgra = std::fs::read(std::env::var("QT_OCR_VI_SAMPLE").unwrap()).unwrap();
        let size = std::env::var("QT_OCR_VI_SIZE").unwrap();
        let (width, height) = size.split_once('x').unwrap();
        let (width, height): (i32, i32) = (width.parse().unwrap(), height.parse().unwrap());
        let words = [Rect { left: 4, top: 8, right: width - 4, bottom: height - 8 }];
        let text = read_line(&root, &bgra, width, height, &words).unwrap();
        println!("OCR: {text}");
        assert!(text.contains("đã cài"), "{text}");
    }

    /// Reads a real screen rectangle: QT_OCR_VI_ROOT as above and
    /// QT_OCR_RECT=<left>,<top>,<right>,<bottom> in physical pixels.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn reads_the_screen() {
        let root = PathBuf::from(std::env::var("QT_OCR_VI_ROOT").unwrap());
        let edges: Vec<i32> = std::env::var("QT_OCR_RECT").unwrap().split(',').map(|v| v.parse().unwrap()).collect();
        let rect = Rect { left: edges[0], top: edges[1], right: edges[2], bottom: edges[3] };
        println!("WINDOWS: {}", crate::capture::join_lines(crate::platform::ocr_lines(rect, None).unwrap()));
        println!("VIETOCR: {}", crate::capture::join_lines(crate::platform::ocr_lines(rect, Some(&root)).unwrap()));
    }
}
