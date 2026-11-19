//! A tiny PNG writer: 8-bit palette, filter 0, zlib with one fixed-Huffman block and a greedy
//! LZ77 that looks back one pixel and one row. Std only, no float, no clock: the same pixels
//! give the same bytes on every host and every toolchain.

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u32; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

struct Bits {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Bits {
    /// Append the low `n` bits of `v`, least significant first.
    fn put(&mut self, v: u32, n: u32) {
        self.acc |= v << self.n;
        self.n += n;
        while self.n >= 8 {
            self.out.push((self.acc & 0xff) as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }

    /// A Huffman code goes out most significant bit first.
    fn code(&mut self, code: u32, n: u32) {
        self.put(code.reverse_bits() >> (32 - n), n);
    }

    fn symbol(&mut self, s: u32) {
        match s {
            0..=143 => self.code(0x30 + s, 8),
            144..=255 => self.code(0x190 + s - 144, 9),
            256..=279 => self.code(s - 256, 7),
            _ => self.code(0xc0 + s - 280, 8),
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.put(0, 7);
        self.out
    }
}

fn index_of(base: &[u16], v: usize) -> usize {
    base.iter().rposition(|&b| usize::from(b) <= v).unwrap_or(0)
}

/// Raw DEFLATE: one final block, fixed Huffman codes.
fn deflate(data: &[u8], row: usize) -> Vec<u8> {
    let mut b = Bits {
        out: Vec::new(),
        acc: 0,
        n: 0,
    };
    b.put(1, 1);
    b.put(1, 2);
    let mut i = 0;
    while i < data.len() {
        let (mut best_len, mut best_dist) = (0, 0);
        for d in [1, row] {
            if d <= i {
                let mut len = 0;
                while i + len < data.len() && len < 258 && data[i + len] == data[i + len - d] {
                    len += 1;
                }
                if len > best_len {
                    (best_len, best_dist) = (len, d);
                }
            }
        }
        if best_len >= 3 {
            let l = index_of(&LEN_BASE, best_len);
            b.symbol(257 + l as u32);
            b.put((best_len - usize::from(LEN_BASE[l])) as u32, LEN_EXTRA[l]);
            let d = index_of(&DIST_BASE, best_dist);
            b.code(d as u32, 5);
            b.put(
                (best_dist - usize::from(DIST_BASE[d])) as u32,
                DIST_EXTRA[d],
            );
            i += best_len;
        } else {
            b.symbol(u32::from(data[i]));
            i += 1;
        }
    }
    b.symbol(256);
    b.finish()
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut s) = (1u32, 0u32);
    for &x in data {
        a = (a + u32::from(x)) % 65521;
        s = (s + a) % 65521;
    }
    (s << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &x in data {
        c ^= u32::from(x);
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    let mut tagged = kind.to_vec();
    tagged.extend_from_slice(body);
    out.extend_from_slice(&tagged);
    out.extend_from_slice(&crc32(&tagged).to_be_bytes());
}

/// Encode `pixels` (one palette index per pixel, `width * height` of them) as a PNG.
pub fn encode(width: usize, height: usize, palette: &[[u8; 3]], pixels: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width + 1) * height);
    for line in pixels.chunks(width) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut z = vec![0x78, 0x01];
    z.extend(deflate(&raw, width + 1));
    z.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 3, 0, 0, 0]);
    let plte: Vec<u8> = palette.iter().flatten().copied().collect();

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"PLTE", &plte);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inflate a fixed-Huffman stream with an independent, naive decoder (test only).
    fn inflate(z: &[u8]) -> Vec<u8> {
        let bits: Vec<u8> = z[2..]
            .iter()
            .flat_map(|b| (0..8).map(move |i| (b >> i) & 1))
            .collect();
        let mut p = 3;
        let take = |n: usize, p: &mut usize| {
            let v = (0..n).fold(0usize, |a, k| a | (usize::from(bits[*p + k]) << k));
            *p += n;
            v
        };
        let huff = |p: &mut usize| {
            let mut c = 0usize;
            (1..=9)
                .find_map(|n| {
                    c = (c << 1) | take(1, p);
                    match (n, c) {
                        (7, 0..=23) => Some(256 + c),
                        (8, 0x30..=0xbf) => Some(c - 0x30),
                        (8, 0xc0..=0xc7) => Some(280 + c - 0xc0),
                        (9, 0x190..=0x1ff) => Some(144 + c - 0x190),
                        _ => None,
                    }
                })
                .expect("a valid fixed-Huffman code")
        };
        let mut out: Vec<u8> = Vec::new();
        loop {
            let s = huff(&mut p);
            match s {
                256 => return out,
                0..=255 => out.push(s as u8),
                _ => {
                    let l = s - 257;
                    let len = usize::from(LEN_BASE[l]) + take(LEN_EXTRA[l] as usize, &mut p);
                    let d = take(5, &mut p).reverse_bits() >> (usize::BITS - 5);
                    let dist = usize::from(DIST_BASE[d]) + take(DIST_EXTRA[d] as usize, &mut p);
                    for _ in 0..len {
                        out.push(out[out.len() - dist]);
                    }
                }
            }
        }
    }

    #[test]
    fn deflate_round_trips_literals_runs_and_row_repeats() {
        let row = 40;
        let mut data = Vec::new();
        for r in 0..30u8 {
            data.push(0);
            for x in 0..39u8 {
                data.push(if r % 3 == 0 {
                    x.wrapping_mul(r)
                } else {
                    (x / 8) * 50
                });
            }
        }
        data.extend(std::iter::repeat_n(7u8, 600));
        data.extend((0..200u32).map(|i| (i * 131 % 251) as u8));
        let mut z = vec![0x78, 0x01];
        z.extend(deflate(&data, row));
        assert_eq!(inflate(&z), data);
        assert!(z.len() < data.len());
    }

    #[test]
    fn checksums_match_known_values() {
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn a_tiny_image_is_a_well_formed_png() {
        let png = encode(2, 2, &[[0, 0, 0], [255, 255, 255]], &[0, 1, 1, 0]);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 13, 10, 26, 10]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }
}
