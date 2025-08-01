// source code take from
// https://github.com/image-rs/image/blob/main/src/codecs/dxt.rs
//!  Decoding of DXT (S3TC) compression
//!
//!  DXT is an image format that supports lossy compression
//!
//!  # Related Links
//!  * <https://www.khronos.org/registry/OpenGL/extensions/EXT/EXT_texture_compression_s3tc.txt> - Description of the DXT compression OpenGL extensions.
//!
//!  Note: this module only implements bare DXT encoding/decoding, it does not parse formats that can contain DXT files like .dds

/**
 * Actual encoding/decoding logic below.
 */
type Rgb = [u8; 3];

/// decodes a 5-bit R, 6-bit G, 5-bit B 16-bit packed color value into 8-bit RGB
/// mapping is done so min/max range values are preserved. So for 5-bit
/// values 0x00 -> 0x00 and 0x1F -> 0xFF
fn enc565_decode(value: u16) -> Rgb {
    let red = (value >> 11) & 0x1F;
    let green = (value >> 5) & 0x3F;
    let blue = (value) & 0x1F;
    [
        (red * 0xFF / 0x1F) as u8,
        (green * 0xFF / 0x3F) as u8,
        (blue * 0xFF / 0x1F) as u8,
    ]
}

/*
 * Functions for decoding DXT compression
 */

/// Constructs the DXT5 alpha lookup table from the two alpha entries
/// if alpha0 > alpha1, constructs a table of [a0, a1, 6 linearly interpolated values from a0 to a1]
/// if alpha0 <= alpha1, constructs a table of [a0, a1, 4 linearly interpolated values from a0 to a1, 0, 0xFF]
fn alpha_table_dxt5(alpha0: u8, alpha1: u8) -> [u8; 8] {
    let mut table = [alpha0, alpha1, 0, 0, 0, 0, 0, 0xFF];
    if alpha0 > alpha1 {
        for i in 2..8u16 {
            table[i as usize] =
                (((8 - i) * u16::from(alpha0) + (i - 1) * u16::from(alpha1)) / 7) as u8;
        }
    } else {
        for i in 2..6u16 {
            table[i as usize] =
                (((6 - i) * u16::from(alpha0) + (i - 1) * u16::from(alpha1)) / 5) as u8;
        }
    }
    table
}

/// decodes an 8-byte dxt color block into the RGB channels of a 16xRGB or 16xRGBA block.
/// source should have a length of 8, dest a length of 48 (RGB) or 64 (RGBA)
fn decode_dxt_colors(source: &[u8], dest: &mut [u8], is_dxt1: bool) {
    // sanity checks, also enable the compiler to elide all following bound checks
    assert!(source.len() == 8 && (dest.len() == 48 || dest.len() == 64));
    // calculate pitch to store RGB values in dest (3 for RGB, 4 for RGBA)
    let pitch = dest.len() / 16;

    // extract color data
    let color0 = u16::from(source[0]) | (u16::from(source[1]) << 8);
    let color1 = u16::from(source[2]) | (u16::from(source[3]) << 8);
    let color_table = u32::from(source[4])
        | (u32::from(source[5]) << 8)
        | (u32::from(source[6]) << 16)
        | (u32::from(source[7]) << 24);
    // let color_table = source[4..8].iter().rev().fold(0, |t, &b| (t << 8) | b as u32);

    // decode the colors to rgb format
    let mut colors = [[0; 3]; 4];
    colors[0] = enc565_decode(color0);
    colors[1] = enc565_decode(color1);

    // determine color interpolation method
    if color0 > color1 || !is_dxt1 {
        // linearly interpolate the other two color table entries
        for i in 0..3 {
            colors[2][i] = ((u16::from(colors[0][i]) * 2 + u16::from(colors[1][i]) + 1) / 3) as u8;
            colors[3][i] = ((u16::from(colors[0][i]) + u16::from(colors[1][i]) * 2 + 1) / 3) as u8;
        }
    } else {
        // linearly interpolate one other entry, keep the other at 0
        for i in 0..3 {
            colors[2][i] = (u16::from(colors[0][i]) + u16::from(colors[1][i])).div_ceil(2) as u8;
        }
    }

    // serialize the result. Every color is determined by looking up
    // two bits in color_table which identify which color to actually pick from the 4 possible colors
    for i in 0..16 {
        dest[i * pitch..i * pitch + 3]
            .copy_from_slice(&colors[(color_table >> (i * 2)) as usize & 3]);
    }
}

/// Decodes a 16-byte bock of dxt5 data to a 16xRGBA block
fn decode_dxt5_block(source: &[u8], dest: &mut [u8]) {
    assert!(source.len() == 16 && dest.len() == 64);

    // extract alpha index table (stored as little endian 64-bit value)
    let alpha_table = source[2..8]
        .iter()
        .rev()
        .fold(0, |t, &b| (t << 8) | u64::from(b));

    // alpha level decode
    let alphas = alpha_table_dxt5(source[0], source[1]);

    // serialize alpha
    for i in 0..16 {
        dest[i * 4 + 3] = alphas[(alpha_table >> (i * 3)) as usize & 7];
    }

    // handle colors
    decode_dxt_colors(&source[8..16], dest, false);
}

/// Decodes a 16-byte bock of dxt3 data to a 16xRGBA block
fn decode_dxt3_block(source: &[u8], dest: &mut [u8]) {
    assert!(source.len() == 16 && dest.len() == 64);

    // extract alpha index table (stored as little endian 64-bit value)
    let alpha_table = source[0..8]
        .iter()
        .rev()
        .fold(0, |t, &b| (t << 8) | u64::from(b));

    // serialize alpha (stored as 4-bit values)
    for i in 0..16 {
        dest[i * 4 + 3] = ((alpha_table >> (i * 4)) as u8 & 0xF) * 0x11;
    }

    // handle colors
    decode_dxt_colors(&source[8..16], dest, false);
}

/// Decode a row of DXT3 data to four rows of RGBA data.
/// `source.len()` should be a multiple of 16, otherwise this panics.
pub(crate) fn decode_dxt3_row(source: &[u8], dest: &mut [u8]) {
    assert!(source.len() % 16 == 0);
    let block_count = source.len() / 16;
    assert!(dest.len() >= block_count * 64);

    // contains the 16 decoded pixels per block
    let mut decoded_block = [0u8; 64];

    for (x, encoded_block) in source.chunks(16).enumerate() {
        decode_dxt3_block(encoded_block, &mut decoded_block);

        // copy the values from the decoded block to linewise RGB layout
        for line in 0..4 {
            let offset = (block_count * line + x) * 16;
            dest[offset..offset + 16].copy_from_slice(&decoded_block[line * 16..(line + 1) * 16]);
        }
    }
}

/// Decode a row of DXT5 data to four rows of RGBA data.
/// `source.len()` should be a multiple of 16, otherwise this panics.
pub(crate) fn decode_dxt5_row(source: &[u8], dest: &mut [u8]) {
    assert!(source.len() % 16 == 0);
    let block_count = source.len() / 16;
    assert!(dest.len() >= block_count * 64);

    // contains the 16 decoded pixels per block
    let mut decoded_block = [0u8; 64];

    for (x, encoded_block) in source.chunks(16).enumerate() {
        decode_dxt5_block(encoded_block, &mut decoded_block);

        // copy the values from the decoded block to linewise RGB layout
        for line in 0..4 {
            let offset = (block_count * line + x) * 16;
            dest[offset..offset + 16].copy_from_slice(&decoded_block[line * 16..(line + 1) * 16]);
        }
    }
}
