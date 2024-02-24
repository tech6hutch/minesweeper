pub fn lerp_colors(min: u32, max: u32, amt: f32) -> u32 {
    let min_bytes = min.to_be_bytes();
    let max_bytes = max.to_be_bytes();
    u32::from_be_bytes([
        0, // always zero
        lerp_u8(min_bytes[1], max_bytes[1], amt),
        lerp_u8(min_bytes[2], max_bytes[2], amt),
        lerp_u8(min_bytes[3], max_bytes[3], amt),
    ])
}

fn lerp_u8(min: u8, max: u8, amt: f32) -> u8 {
    let min = i16::from(min);
    let max = i16::from(max);
    (min + ((max - min) as f32 * amt) as i16) as u8
}
