//! Native pure-Rust Luby Transform (LT) Fountain QR encoder compatible with https://qrs.inled.es.
//! Zero C-library or WebKit dependencies.

use qrcode::{EcLevel, QrCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const TYPE_HEADER: u8 = 1;
pub const TYPE_DATA: u8 = 2;
pub const DATA_OVERHEAD: usize = 17;
pub const HEADER_EVERY: usize = 10;

const MAGIC0: u8 = 0x46; // 'F'
const MAGIC1: u8 = 0x51; // 'Q'

const SOLITON_C: f64 = 0.05;
const SOLITON_DELTA: f64 = 1.0;
const SIMPLE_FRAME_RATE: f64 = 0.05;

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c: u32 = 0xffff_ffff;
    for &b in bytes {
        let mut entry = (c ^ (b as u32)) & 0xff;
        for _ in 0..8 {
            entry = if (entry & 1) != 0 {
                0xedb8_8320 ^ (entry >> 1)
            } else {
                entry >> 1
            };
        }
        c = entry ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

pub struct Xorshift32 {
    s: u32,
}

impl Xorshift32 {
    pub fn new(seed: u32) -> Self {
        let s = if seed == 0 { 0x9e37_79b9 } else { seed };
        Self { s }
    }

    pub fn next(&mut self) -> u32 {
        self.s ^= self.s << 13;
        self.s ^= self.s >> 17;
        self.s ^= self.s << 5;
        self.s
    }
}

pub fn build_cdf(k: usize) -> Vec<f64> {
    let mut cdf = vec![0.0; k + 1];
    if k <= 1 {
        cdf[k] = 1.0;
        return cdf;
    }
    let k_f = k as f64;
    let c = SOLITON_C;
    let delta = SOLITON_DELTA;
    let r = 1.0f64.max(c * (k_f / delta).ln() * k_f.sqrt());
    let spike = 1usize.max((k as usize).min((k_f / r).floor() as usize));
    let mut p = vec![0.0; k + 1];

    p[1] += 1.0 / k_f;
    for d in 2..=k {
        let d_f = d as f64;
        p[d] += 1.0 / (d_f * (d_f - 1.0));
    }
    for d in 1..spike {
        let d_f = d as f64;
        p[d] += r / (d_f * k_f);
    }
    p[spike] += 0.0f64.max((r * (r / delta).ln()) / k_f);

    let mut acc = 0.0;
    for d in 1..=k {
        acc += p[d];
        cdf[d] = acc;
    }
    for d in 1..=k {
        cdf[d] /= acc;
    }
    cdf[k] = 1.0;
    cdf
}

fn sample_degree(rng: &mut Xorshift32, cdf: &[f64]) -> usize {
    let u = (rng.next() as f64) / 4_294_967_296.0;
    let mut lo = 1;
    let mut hi = cdf.len() - 1;
    while lo < hi {
        let mid = (lo + hi) >> 1;
        if cdf[mid] <= u {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

pub fn frame_plan(k: usize, cdf: &[f64], seed: u32, file_id: u32) -> Vec<usize> {
    let mut rng = Xorshift32::new(seed ^ file_id);
    let simple = ((rng.next() as f64) / 4_294_967_296.0) < SIMPLE_FRAME_RATE;
    let degree = if simple { 1 } else { sample_degree(&mut rng, cdf) };
    let mut picked = Vec::with_capacity(degree.min(k));
    let mut used = std::collections::HashSet::new();
    while picked.len() < degree && picked.len() < k {
        let index = (rng.next() as usize) % k;
        if used.insert(index) {
            picked.push(index);
        }
    }
    picked
}

pub fn bytes_to_base64_url(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };

        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);

        if i + 1 < bytes.len() {
            out.push(TABLE[((triple >> 6) & 0x3f) as usize] as char);
        }
        if i + 2 < bytes.len() {
            out.push(TABLE[(triple & 0x3f) as usize] as char);
        }

        i += 3;
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FountainMeta {
    pub n: String,
    pub m: String,
    pub s: usize,
    pub b: usize,
    pub k: usize,
    pub h: String,
    pub sp: String,
    pub fps: u32,
}

pub struct FountainEncoder {
    pub id: u32,
    pub meta: FountainMeta,
    pub block_size: usize,
    pub k: usize,
    pub cdf: Vec<f64>,
    pub blocks: Vec<u8>,
    pub seq: u16,
    pub frame_index: usize,
    header: Vec<u8>,
}

impl FountainEncoder {
    pub fn new(data: &[u8], file_name: &str, speed: &str) -> Self {
        let (block_size, fps) = match speed {
            "low" => (160, 4),
            "fast" => (420, 12),
            _ => (280, 8), // default "medium"
        };
        let file_len = data.len();
        let k = 1.max((file_len + block_size - 1) / block_size);
        let cdf = build_cdf(k);

        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash_bytes = hasher.finalize();
        let hash_b64 = bytes_to_base64_url(&hash_bytes);

        let meta = FountainMeta {
            n: file_name.to_string(),
            m: "text/plain".to_string(),
            s: file_len,
            b: block_size,
            k,
            h: hash_b64,
            sp: speed.to_string(),
            fps,
        };

        // Pad data to k * block_size
        let mut blocks = vec![0u8; k * block_size];
        blocks[..file_len].copy_from_slice(data);

        // Pseudorandom file ID (can use timestamp/crc or simple seed)
        let file_id = (crc32(data) ^ 0x5a5a_a5a5) | 1;

        let json = serde_json::to_vec(&meta).unwrap_or_default();
        let target_len = (7 + json.len()).max(DATA_OVERHEAD + block_size);
        let mut header = vec![0u8; target_len];
        header[0] = MAGIC0;
        header[1] = MAGIC1;
        header[2] = TYPE_HEADER;
        header[3..7].copy_from_slice(&file_id.to_le_bytes());
        header[7..7 + json.len()].copy_from_slice(&json);

        Self {
            id: file_id,
            meta,
            block_size,
            k,
            cdf,
            blocks,
            seq: 0,
            frame_index: 0,
            header,
        }
    }

    pub fn next_frame(&mut self) -> Vec<u8> {
        if self.frame_index % HEADER_EVERY == 0 {
            self.frame_index += 1;
            return self.header.clone();
        }
        self.frame_index += 1;

        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);

        // Deterministic pseudo-random seed based on seq & frame_index
        let seed = (((self.frame_index as u64 * 1_103_515_245 + 12_345) & 0xffff_ffff) as u32) | 7;

        let mut payload = vec![0u8; self.block_size];
        for index in frame_plan(self.k, &self.cdf, seed, self.id) {
            let offset = index * self.block_size;
            for j in 0..self.block_size {
                payload[j] ^= self.blocks[offset + j];
            }
        }

        let mut out = vec![0u8; DATA_OVERHEAD + payload.len()];
        out[0] = MAGIC0;
        out[1] = MAGIC1;
        out[2] = TYPE_DATA;
        out[3..7].copy_from_slice(&self.id.to_le_bytes());
        out[7..9].copy_from_slice(&seq.to_le_bytes());
        out[9..13].copy_from_slice(&seed.to_le_bytes());
        out[13..17].copy_from_slice(&crc32(&payload).to_le_bytes());
        out[DATA_OVERHEAD..].copy_from_slice(&payload);
        out
    }

    pub fn next_qr(&mut self) -> Result<QrCode, String> {
        let frame = self.next_frame();
        QrCode::with_error_correction_level(&frame, EcLevel::L)
            .map_err(|e| format!("Failed to generate QR code: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32() {
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
    }

    #[test]
    fn test_fountain_encoder_generation() {
        let sample = b"2026-11-15 test log contents for pulsar recovery fountain transfer";
        let mut encoder = FountainEncoder::new(sample, "test.log", "medium");

        // First frame must be header
        let h_frame = encoder.next_frame();
        assert_eq!(h_frame[0], MAGIC0);
        assert_eq!(h_frame[1], MAGIC1);
        assert_eq!(h_frame[2], TYPE_HEADER);

        // Next frame must be data
        let d_frame = encoder.next_frame();
        assert_eq!(d_frame[0], MAGIC0);
        assert_eq!(d_frame[1], MAGIC1);
        assert_eq!(d_frame[2], TYPE_DATA);

        // Next QR generation
        let qr = encoder.next_qr();
        assert!(qr.is_ok());

        // Stream 50 consecutive frames and generate QRs
        for _ in 0..50 {
            let next_qr = encoder.next_qr();
            assert!(next_qr.is_ok());
        }
    }
}
