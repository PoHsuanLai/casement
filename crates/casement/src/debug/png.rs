//! A PNG writer with no compression (stored deflate blocks): the debug `Shot` verb writes
//! frames with it so casement's binary needs no image crate. Files are large and exact.

/// The 8-byte PNG signature.
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
/// The most a stored deflate block holds.
const STORED_MAX: usize = 65_535;

/// The CRC-32 (IEEE) of `bytes`, as PNG chunks carry it.
fn crc32(bytes: &[u8]) -> u32 {
    let table: Vec<u32> = (0..256_u32)
        .map(|n| {
            (0..8).fold(n, |c, _| {
                if c & 1 == 1 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                }
            })
        })
        .collect();
    !bytes.iter().fold(!0_u32, |c, byte| {
        table[usize::from((c ^ u32::from(*byte)) as u8)] ^ (c >> 8)
    })
}

/// The Adler-32 of `bytes`, as zlib streams carry it.
fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let (a, b) = bytes.iter().fold((1_u32, 0_u32), |(a, b), byte| {
        let a = (a + u32::from(*byte)) % MOD;
        (a, (b + a) % MOD)
    });
    b << 16 | a
}

/// One chunk: length, type, data, CRC.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend(u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    let from = out.len();
    out.extend(kind);
    out.extend(data);
    let crc = crc32(&out[from..]);
    out.extend(crc.to_be_bytes());
}

/// `rgba` (rows top to bottom, four bytes a pixel) as an RGBA PNG, `width` x `height`.
pub(super) fn encode_rgba(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let stride = width as usize * 4;
    let raw: Vec<u8> = rgba
        .chunks(stride.max(1))
        .flat_map(|row| std::iter::once(0_u8).chain(row.iter().copied()))
        .collect();
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(STORED_MAX).collect();
    for (at, block) in blocks.iter().enumerate() {
        let last = u8::from(at + 1 == blocks.len());
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        zlib.push(last);
        zlib.extend(len.to_le_bytes());
        zlib.extend((!len).to_le_bytes());
        zlib.extend(*block);
    }
    zlib.extend(adler32(&raw).to_be_bytes());
    let mut header = Vec::new();
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    header.extend([8, 6, 0, 0, 0]);
    let mut out = SIGNATURE.to_vec();
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksums_match_their_published_vectors() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn a_two_pixel_image_has_the_expected_chunks() {
        let png = encode_rgba(2, 1, &[1, 2, 3, 255, 4, 5, 6, 255]);
        assert_eq!(png[..8], SIGNATURE);
        // IHDR: 13 bytes of data after length and type.
        assert_eq!(&png[8..16], &[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
        assert_eq!(&png[16..24], &[0, 0, 0, 2, 0, 0, 0, 1]);
        assert_eq!(&png[24..29], &[8, 6, 0, 0, 0]);
        // The stored block holds the filter byte and the eight pixel bytes, verbatim.
        let idat = png
            .windows(4)
            .position(|w| w == b"IDAT")
            .expect("an IDAT chunk");
        let zlib = &png[idat + 4..];
        assert_eq!(&zlib[..2], &[0x78, 0x01]);
        assert_eq!(zlib[2], 1, "the only block is the last");
        assert_eq!(&zlib[3..7], &[9, 0, 0xf6, 0xff]);
        assert_eq!(&zlib[7..16], &[0, 1, 2, 3, 255, 4, 5, 6, 255]);
        assert!(png.ends_with(&[0xae, 0x42, 0x60, 0x82]), "the IEND crc");
    }

    #[test]
    fn a_large_image_splits_into_stored_blocks() {
        let png = encode_rgba(100, 200, &vec![7; 100 * 200 * 4]);
        let idat = png
            .windows(4)
            .position(|w| w == b"IDAT")
            .expect("an IDAT chunk");
        // 200 rows of 401 bytes: 80200 bytes, two blocks, the first full and not final.
        assert_eq!(png[idat + 4 + 2], 0);
        assert_eq!(&png[idat + 4 + 3..idat + 4 + 7], &[0xff, 0xff, 0, 0]);
    }
}
