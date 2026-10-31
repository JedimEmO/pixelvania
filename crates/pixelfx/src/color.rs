//! RGBA colors and discrete palette ramps, ported from pixelfx core/color.js.
use crate::math::{clamp, round};
pub type Color = [u8; 4];
pub const WHITE: Color = [255, 255, 255, 255];
pub const CLEAR: Color = [0; 4];
pub const fn hex(n: u32) -> Color {
    [(n >> 16) as u8, (n >> 8) as u8, n as u8, 255]
}
pub fn mix(a: Color, b: Color, t: f64) -> Color {
    let t = clamp(t);
    let mut c = WHITE;
    for i in 0..3 {
        c[i] = (a[i] as f64 + (b[i] as f64 - a[i] as f64) * t) as u8;
    }
    c
}
pub fn scale(c: Color, k: f64) -> Color {
    let mut out = WHITE;
    for i in 0..3 {
        out[i] = (c[i] as f64 * k).min(255.0) as u8;
    }
    out
}
/// Resamples a nonempty palette while retaining both end colors.
pub fn fit(r: &[Color], n: usize) -> Vec<Color> {
    assert!(!r.is_empty(), "palette must not be empty");
    (0..n)
        .map(|i| {
            r[round((i * (r.len() - 1)) as f64 / (n.saturating_sub(1).max(1)) as f64) as usize]
        })
        .collect()
}
pub fn at(r: &[Color], t: f64) -> Color {
    r[((t * r.len() as f64).floor().max(0.0) as usize).min(r.len() - 1)]
}
pub const FIRE: &[Color] = &[
    hex(0x1f0b10),
    hex(0x4a1118),
    hex(0x8c1d1c),
    hex(0xcc3a1a),
    hex(0xf2701f),
    hex(0xffae35),
    hex(0xffe27a),
    hex(0xfff8d6),
];
pub const EMBER: &[Color] = &[
    hex(0x1a0a0e),
    hex(0x3d1012),
    hex(0x701a14),
    hex(0xa8301a),
    hex(0xe0561e),
    hex(0xff8a2a),
    hex(0xffc46a),
];
pub const LAVA: &[Color] = &[
    hex(0x14060a),
    hex(0x380a10),
    hex(0x6e1210),
    hex(0xac2410),
    hex(0xe8501a),
    hex(0xff8c2a),
    hex(0xffd25a),
];
pub const SMOKE: &[Color] = &[
    hex(0x0e0c12),
    hex(0x1d1a22),
    hex(0x2e2a35),
    hex(0x433e4b),
    hex(0x5c5664),
    hex(0x7b7484),
    hex(0xa39cab),
];
pub const DUST: &[Color] = &[
    hex(0x1c1610),
    hex(0x33281c),
    hex(0x4f3f2c),
    hex(0x6e5a40),
    hex(0x927a58),
    hex(0xb89f78),
    hex(0xdcc8a2),
];
pub const ICE: &[Color] = &[
    hex(0x0c1430),
    hex(0x162c5c),
    hex(0x1f4f94),
    hex(0x2f7fcc),
    hex(0x58b4ec),
    hex(0x9bdcff),
    hex(0xd8f4ff),
    hex(0xffffff),
];
pub const WATER: &[Color] = &[
    hex(0x081830),
    hex(0x0e2e5a),
    hex(0x14508c),
    hex(0x1c76bc),
    hex(0x36a2e0),
    hex(0x78ccf4),
    hex(0xc4ecff),
    hex(0xffffff),
];
pub const ELECTRIC: &[Color] = &[
    hex(0x0a0f2e),
    hex(0x172a78),
    hex(0x2550d0),
    hex(0x3f8cff),
    hex(0x7cc8ff),
    hex(0xc8f0ff),
    hex(0xffffff),
];
pub const VOLT: &[Color] = &[
    hex(0x24160a),
    hex(0x5e3a0c),
    hex(0xa8680f),
    hex(0xe8a218),
    hex(0xffd23c),
    hex(0xfff08a),
    hex(0xffffff),
];
pub const POISON: &[Color] = &[
    hex(0x0d1a0c),
    hex(0x173a14),
    hex(0x265e18),
    hex(0x3f8a1e),
    hex(0x6cb82a),
    hex(0xa8e04a),
    hex(0xe0ff8a),
];
pub const HOLY: &[Color] = &[
    hex(0x2a1c08),
    hex(0x5a3e10),
    hex(0x9a6c18),
    hex(0xd8a428),
    hex(0xffd84a),
    hex(0xfff09a),
    hex(0xfffbe6),
    hex(0xffffff),
];
pub const HEAL: &[Color] = &[
    hex(0x08201a),
    hex(0x0f3f2c),
    hex(0x17663e),
    hex(0x24924e),
    hex(0x3ec466),
    hex(0x7ee88a),
    hex(0xc8ffc4),
    hex(0xffffff),
];
pub const ARCANE: &[Color] = &[
    hex(0x140a26),
    hex(0x2b1252),
    hex(0x481c88),
    hex(0x6b2cc0),
    hex(0x9a48ec),
    hex(0xc684ff),
    hex(0xecc8ff),
    hex(0xffffff),
];
pub const SHADOW: &[Color] = &[
    hex(0x050308),
    hex(0x0f0a18),
    hex(0x1c1230),
    hex(0x2c1a4a),
    hex(0x432468),
    hex(0x5e3290),
    hex(0x8048b8),
    hex(0xa970e0),
];
pub const BLOOD: &[Color] = &[
    hex(0x14040a),
    hex(0x3a0a14),
    hex(0x6a101c),
    hex(0xa01824),
    hex(0xd42a30),
    hex(0xff5a50),
    hex(0xff9a8a),
];
pub const EARTH: &[Color] = &[
    hex(0x140e0a),
    hex(0x2a1e14),
    hex(0x47321e),
    hex(0x69492a),
    hex(0x8e6638),
    hex(0xb88a52),
    hex(0xdcb47c),
];
pub const STONE: &[Color] = &[
    hex(0x0e0e12),
    hex(0x1c1c24),
    hex(0x2c2c38),
    hex(0x42424f),
    hex(0x5c5c6b),
    hex(0x7d7d8c),
    hex(0xa4a4b2),
    hex(0xd0d0da),
];
pub const GOLD: &[Color] = &[
    hex(0x1e1206),
    hex(0x4a2c08),
    hex(0x86520c),
    hex(0xc48412),
    hex(0xf0b424),
    hex(0xffe066),
    hex(0xfff6c4),
    hex(0xffffff),
];
pub const LEAF: &[Color] = &[
    hex(0x0c1a0e),
    hex(0x163418),
    hex(0x225224),
    hex(0x357530),
    hex(0x4f9a3a),
    hex(0x7cc048),
    hex(0xb4e070),
    hex(0xe4ffb0),
];
pub const WOOD: &[Color] = &[
    hex(0x160c06),
    hex(0x2e1a0c),
    hex(0x4a2c14),
    hex(0x6a421e),
    hex(0x8e5c2c),
    hex(0xb47c40),
    hex(0xd8a464),
];
pub const METAL: &[Color] = &[
    hex(0x0c0e14),
    hex(0x1c2230),
    hex(0x2e3648),
    hex(0x4a5468),
    hex(0x6e7a90),
    hex(0x9aa6ba),
    hex(0xd0d8e6),
    hex(0xffffff),
];
pub const BONE: &[Color] = &[
    hex(0x1a1814),
    hex(0x3a352c),
    hex(0x5e5648),
    hex(0x8a806c),
    hex(0xb8ad96),
    hex(0xe4dcc8),
    hex(0xfffaf0),
];
pub const NEON: &[Color] = &[
    hex(0x1a0516),
    hex(0x3d0a33),
    hex(0x701560),
    hex(0xb0228f),
    hex(0xf53cd1),
    hex(0xff86e6),
    hex(0xffd2f6),
    hex(0xffffff),
];
pub const CYBER: &[Color] = &[
    hex(0x041418),
    hex(0x08303a),
    hex(0x0c5260),
    hex(0x108088),
    hex(0x18b4b0),
    hex(0x4ce8d8),
    hex(0xb4fff0),
    hex(0xffffff),
];
pub const MATRIX: &[Color] = &[
    hex(0x020a04),
    hex(0x05200c),
    hex(0x0a3c16),
    hex(0x106624),
    hex(0x1a9a34),
    hex(0x3cd650),
    hex(0xa4ffa8),
    hex(0xe8ffe8),
];
pub const CANDY: &[Color] = &[
    hex(0xff4a5a),
    hex(0xff9a2a),
    hex(0xffe03c),
    hex(0x5ad84a),
    hex(0x3cc8ff),
    hex(0x7a6cff),
    hex(0xf53cd1),
];
pub const NIGHT: &[Color] = &[
    hex(0x07060d),
    hex(0x0c0a18),
    hex(0x131026),
    hex(0x1b1734),
    hex(0x241f44),
    hex(0x2f2956),
];
