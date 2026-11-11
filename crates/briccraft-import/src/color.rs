use briccraft_parts::PALETTE;

pub(crate) fn nearest(rgb: [f64; 3]) -> u8 {
    PALETTE
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let distance = |color: &[u8; 3]| {
                (0..3)
                    .map(|i| {
                        let d = color[i] as f64 - rgb[i].clamp(0.0, 1.0) * 255.0;
                        d * d * [2.0, 4.0, 3.0][i]
                    })
                    .sum::<f64>()
            };
            distance(a).total_cmp(&distance(b))
        })
        .map_or(0, |(i, _)| i as u8)
}
pub(crate) fn linear_to_srgb(value: f64) -> f64 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}
pub(crate) fn srgb_to_linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
pub(crate) fn tag_color(value: &str) -> Option<u8> {
    let value = value.trim().to_ascii_lowercase();
    let rgb = if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 6 && hex.is_ascii() {
            [
                u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0,
                u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0,
                u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0,
            ]
        } else {
            return None;
        }
    } else {
        match value.as_str() {
            "white" => [0.96, 0.96, 0.93],
            "black" => [0.1, 0.1, 0.1],
            "red" => [0.75, 0.12, 0.1],
            "brown" => [0.42, 0.24, 0.1],
            "grey" | "gray" | "silver" => [0.65, 0.65, 0.65],
            "lightgrey" | "lightgray" => [0.8, 0.8, 0.8],
            "darkgrey" | "darkgray" => [0.36, 0.36, 0.36],
            "beige" | "sand" => [0.84, 0.77, 0.60],
            "blue" => [0.06, 0.41, 0.67],
            "green" => [0.15, 0.48, 0.24],
            "yellow" => [0.96, 0.8, 0.18],
            "orange" => [0.86, 0.52, 0.25],
            _ => return None,
        }
    };
    Some(nearest(rgb))
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_unicode_hex_does_not_slice_inside_a_character() {
        assert_eq!(super::tag_color("#中文"), None);
        assert_eq!(super::tag_color("#zzzzzz"), None);
        assert_eq!(super::tag_color("#C4281C"), Some(0));
    }
}
