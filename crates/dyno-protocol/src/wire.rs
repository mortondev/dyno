//! How pack files travel (docs/design/data-and-streaming.md §1): the server
//! keeps each file encoded with the smallest of a few lossless codecs, and
//! the client decodes it, checks the hash of the original bytes and caches
//! the original. Files are always named by the hash of their original bytes.

/// Sent as they are (PNGs and other files that do not shrink).
pub const RAW: u8 = 0;
/// Raw deflate (no zlib header).
pub const DEFLATE: u8 = 1;
/// A 16-bit PCM WAV: each sample less its order-2 prediction, split into
/// low and high byte planes, then deflated. Lossless.
pub const PCM16: u8 = 2;

/// Where a WAV's 16-bit PCM samples are: (start, length in bytes, channels).
fn pcm16(b: &[u8]) -> Option<(usize, usize, usize)> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let mut channels = None;
    while at + 8 <= b.len() {
        let id = &b[at..at + 4];
        let len = u32::from_le_bytes(b[at + 4..at + 8].try_into().ok()?) as usize;
        let body = at + 8;
        if id == b"fmt " && len >= 16 && body + 16 <= b.len() {
            let format = u16::from_le_bytes([b[body], b[body + 1]]);
            let ch = u16::from_le_bytes([b[body + 2], b[body + 3]]) as usize;
            let bits = u16::from_le_bytes([b[body + 14], b[body + 15]]);
            if format != 1 || bits != 16 || ch == 0 {
                return None;
            }
            channels = Some(ch);
        } else if id == b"data" {
            let len = len.min(b.len() - body);
            let ch = channels?;
            return Some((body, len - len % (2 * ch), ch));
        }
        at = body + len + (len & 1);
    }
    None
}

/// Order-2 prediction of a channel's next sample.
#[inline]
fn predict(p1: i16, p2: i16) -> i16 {
    (p1 as i32 * 2 - p2 as i32) as i16
}

/// Replace a WAV's samples by their residuals in two byte planes.
fn pcm_forward(b: &[u8]) -> Option<Vec<u8>> {
    let (start, len, ch) = pcm16(b)?;
    let n = len / 2;
    let mut out = b.to_vec();
    let mut hist = vec![(0i16, 0i16); ch];
    for i in 0..n {
        let x = i16::from_le_bytes([b[start + 2 * i], b[start + 2 * i + 1]]);
        let h = &mut hist[i % ch];
        let e = x.wrapping_sub(predict(h.0, h.1)) as u16;
        *h = (x, h.0);
        out[start + i] = e as u8;
        out[start + n + i] = (e >> 8) as u8;
    }
    Some(out)
}

/// Undo [`pcm_forward`] in place.
fn pcm_inverse(b: &mut [u8]) -> Option<()> {
    let (start, len, ch) = pcm16(b)?;
    let n = len / 2;
    let planes = b[start..start + len].to_vec();
    let mut hist = vec![(0i16, 0i16); ch];
    for i in 0..n {
        let e = u16::from_le_bytes([planes[i], planes[n + i]]) as i16;
        let h = &mut hist[i % ch];
        let x = e.wrapping_add(predict(h.0, h.1));
        *h = (x, h.0);
        b[start + 2 * i..start + 2 * i + 2].copy_from_slice(&x.to_le_bytes());
    }
    Some(())
}

/// The smallest encoding of a file: (codec, bytes on the wire).
pub fn encode(bytes: &[u8]) -> (u8, Vec<u8>) {
    let mut best = (RAW, bytes.to_vec());
    let deflated = miniz_oxide::deflate::compress_to_vec(bytes, 9);
    if deflated.len() < best.1.len() {
        best = (DEFLATE, deflated);
    }
    if let Some(planes) = pcm_forward(bytes) {
        let d = miniz_oxide::deflate::compress_to_vec(&planes, 9);
        if d.len() < best.1.len() {
            best = (PCM16, d);
        }
    }
    best
}

/// The original bytes of a file of `size` bytes from its encoding, or None
/// when the data is damaged or the codec unknown.
pub fn decode(codec: u8, data: &[u8], size: usize) -> Option<Vec<u8>> {
    let inflate = |d: &[u8]| {
        miniz_oxide::inflate::decompress_to_vec_with_limit(d, size)
            .ok()
            .filter(|v| v.len() == size)
    };
    match codec {
        RAW => (data.len() == size).then(|| data.to_vec()),
        DEFLATE => inflate(data),
        PCM16 => {
            let mut b = inflate(data)?;
            pcm_inverse(&mut b)?;
            Some(b)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(samples: &[i16], channels: u16) -> Vec<u8> {
        let mut b = Vec::new();
        let data = samples.len() as u32 * 2;
        b.extend(b"RIFF");
        b.extend((36 + data).to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(channels.to_le_bytes());
        b.extend(44100u32.to_le_bytes());
        b.extend((44100 * 2 * channels as u32).to_le_bytes());
        b.extend((2 * channels).to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend(data.to_le_bytes());
        for s in samples {
            b.extend(s.to_le_bytes());
        }
        b
    }

    #[test]
    fn every_codec_round_trips() {
        // a tone with wrap-around extremes and some noise
        let s: Vec<i16> = (0..20_000)
            .map(|i| {
                let t = i as f32 / 44100.0;
                let v =
                    (t * 440.0 * std::f32::consts::TAU).sin() * 30000.0 + ((i * 7919) % 97) as f32;
                if i % 5000 == 0 { i16::MIN } else { v as i16 }
            })
            .collect();
        for ch in [1, 2] {
            let w = wav(&s, ch);
            let (codec, data) = encode(&w);
            assert_eq!(codec, PCM16);
            assert!(data.len() < w.len() * 3 / 4);
            assert_eq!(decode(codec, &data, w.len()).as_deref(), Some(&w[..]));
        }
        let text = b"{\"a\": [1, 2, 3], \"b\": [1, 2, 3], \"c\": [1, 2, 3]}".repeat(40);
        let (codec, data) = encode(&text);
        assert_eq!(codec, DEFLATE);
        assert_eq!(decode(codec, &data, text.len()).as_deref(), Some(&text[..]));
        // incompressible bytes go as they are
        let noise: Vec<u8> = (0..4000u32)
            .scan(0x9E37_79B9u32, |x, _| {
                *x ^= *x << 13;
                *x ^= *x >> 17;
                *x ^= *x << 5;
                Some((*x >> 24) as u8)
            })
            .collect();
        let (codec, data) = encode(&noise);
        assert_eq!((codec, &data[..]), (RAW, &noise[..]));
        // damage and lies are refused
        assert!(decode(DEFLATE, &[1, 2, 3], 10).is_none());
        assert!(decode(RAW, &noise, noise.len() + 1).is_none());
        assert!(decode(9, &noise, noise.len()).is_none());
    }
}
