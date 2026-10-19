//! Hand-drawn pixel art (as text grids) plus procedurally painted backdrops.
//! Small sprites are painted as rectangles every frame so they can flash and tint;
//! large backdrops are baked once into nearest-filtered egui textures.

use bevy_egui::egui::{self, Color32, ColorImage, Pos2, Rect, Vec2};

use crate::rng::{hash2, hashf};

pub fn pal(c: char) -> Option<[u8; 3]> {
    Some(match c {
        'k' => [13, 15, 18],
        'd' => [42, 47, 54],
        'm' => [74, 82, 92],
        'l' => [124, 135, 146],
        'h' => [184, 194, 201],
        'r' => [200, 50, 42],
        'o' => [224, 122, 42],
        'y' => [242, 200, 74],
        'g' => [126, 240, 138],
        'c' => [95, 211, 230],
        'b' => [122, 74, 42],
        'u' => [74, 46, 30],
        'w' => [244, 241, 224],
        'p' => [138, 74, 208],
        'q' => [84, 44, 130],
        't' => [107, 106, 58],
        's' => [150, 146, 90],
        'n' => [222, 170, 130],
        _ => return None,
    })
}

#[derive(Clone)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<[u8; 3]>>,
}

impl Sprite {
    pub fn parse(rows: &[&str]) -> Self {
        let w = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let h = rows.len();
        let mut px = vec![None; w * h];
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                px[y * w + x] = pal(c);
            }
        }
        Self { w, h, px }
    }

    /// Paint at `min` (top-left). `flash` blends toward white, `tint` multiplies.
    pub fn paint(&self, p: &egui::Painter, min: Pos2, scale: f32, flash: f32, tint: Option<Color32>, flip: bool) {
        for y in 0..self.h {
            for x in 0..self.w {
                let sx = if flip { self.w - 1 - x } else { x };
                if let Some(c) = self.px[y * self.w + sx] {
                    let mut col = [c[0] as f32, c[1] as f32, c[2] as f32];
                    if let Some(t) = tint {
                        col[0] *= t.r() as f32 / 255.0;
                        col[1] *= t.g() as f32 / 255.0;
                        col[2] *= t.b() as f32 / 255.0;
                    }
                    for ch in col.iter_mut() {
                        *ch += (255.0 - *ch) * flash;
                    }
                    let r = Rect::from_min_size(
                        Pos2::new(min.x + x as f32 * scale, min.y + y as f32 * scale),
                        Vec2::splat(scale),
                    );
                    p.rect_filled(r, 0.0, Color32::from_rgb(col[0] as u8, col[1] as u8, col[2] as u8));
                }
            }
        }
    }
}

pub struct SpriteBank {
    pub scout: Sprite,
    pub artillery: Sprite,
    pub carrier: Sprite,
    pub drone: Sprite,
    pub tick: Sprite,
    pub pilot: Sprite,
    pub pilot_face: Sprite,
    pub chief_face: Sprite,
    pub control_face: Sprite,
    pub enemy_face: Sprite,
    pub system_face: Sprite,
    pub icons: Vec<Sprite>,
    pub wrench: Sprite,
}

impl SpriteBank {
    pub fn new() -> Self {
        Self {
            scout: Sprite::parse(SCOUT),
            artillery: Sprite::parse(ARTILLERY),
            carrier: Sprite::parse(CARRIER),
            drone: Sprite::parse(DRONE),
            tick: Sprite::parse(TICK),
            pilot: Sprite::parse(PILOT),
            pilot_face: Sprite::parse(FACE_PILOT),
            chief_face: Sprite::parse(FACE_CHIEF),
            control_face: Sprite::parse(FACE_CONTROL),
            enemy_face: Sprite::parse(FACE_ENEMY),
            system_face: Sprite::parse(FACE_SYSTEM),
            icons: ICONS.iter().map(|i| Sprite::parse(i)).collect(),
            wrench: Sprite::parse(WRENCH),
        }
    }
}

// --------------------------------------------------------------------------------------
// Enemies
// --------------------------------------------------------------------------------------

const SCOUT: &[&str] = &[
    "......kkkkkk..........",
    ".....ktssstk..........",
    "kkkkktsshhstkkkk......",
    "kmmmmmtssssstrrtk.....",
    "kkkkkttttttttrrtk.....",
    ".....kttttttttttk.....",
    "......kdtttttdk.......",
    "......kmk...kmk.......",
    ".....kmk.....kmk......",
    "....kmk.....kmk.......",
    "....kmk....kmk........",
    "...kmk......kmk.......",
    "..kmmk.......kmmk.....",
    ".kkkkk......kkkkk.....",
];

const ARTILLERY: &[&str] = &[
    "........................kk......",
    "......................kkllk.....",
    "....................kkllmk......",
    "..................kkllmk........",
    "................kkllmk..........",
    "..............kkllmk............",
    "...........kkkllmmk.............",
    "......kkkkkbbbmmkkkkkk..........",
    "....kkmmmmmmmmmmmmmmmmkk........",
    "...kmllllllllllllllllllmk.......",
    "...kmmbbmmmmmmmmmmmbbmmmk.......",
    "...kmmmmmmmmyyrmmmmmmmmmk.......",
    "...kkuuuuuuuuuuuuuuuuuuukk......",
    "....kkkkkkkkkkkkkkkkkkkkk.......",
    "...kmk...kmk.....kmk...kmk......",
    "..kmk.....kmk...kmk.....kmk.....",
    ".kmk.......kmk.kmk.......kmk....",
    "kmk.........kmkmk.........kmk...",
    "kkk..........kkk...........kkk..",
];

const CARRIER: &[&str] = &[
    "..........kkkkkkkkkk..........",
    ".......kkkppppppppppkkk.......",
    ".....kkpppqqpppppppqqpppkk....",
    "....kppqqkggkpppppkggkqqppk...",
    "...kpppqqkggkpppppkggkqqpppk..",
    "...kppppqqkkpppppppkkqqppppk..",
    "..kppppppppppppppppppppppppk..",
    "..kmmmmmmmmmmmmmmmmmmmmmmmmmk.",
    ".kmllllllllllllllllllllllllmk.",
    ".kmmmmkkkmmmmmmmmmmmmkkkmmmmk.",
    "..kkkkk.kkkkkkkkkkkkkk.kkkkk..",
    "...kmk...kmk......kmk...kmk...",
    "...kmk...kmk......kmk...kmk...",
    "..kmmk..kmmk......kmmk..kmmk..",
    "..kkkk..kkkk......kkkk..kkkk..",
];

const DRONE: &[&str] = &[
    "k.....k",
    "kmkkkmk",
    ".kpgpk.",
    "kmkkkmk",
    "k.....k",
];

const TICK: &[&str] = &[
    ".k......k.",
    "..k.kk.k..",
    "...kppk...",
    "..kpggpk..",
    ".kpqppqpk.",
    "kpqppppqpk",
    ".kkpqqpkk.",
    "k..k..k..k",
];

const PILOT: &[&str] = &[
    "..kkkk..",
    ".kooook.",
    ".kocckk.",
    ".koooo..",
    "..knnk..",
    ".kttttk.",
    "kttottk.",
    "kntttnk.",
    ".kttttk.",
    ".kdkkdk.",
    ".kdk.kdk",
    "kkk..kkk",
];

const WRENCH: &[&str] = &[
    ".ll.",
    "l..l",
    ".ll.",
    ".ll.",
    ".ll.",
    ".ll.",
];

// --------------------------------------------------------------------------------------
// Radio portraits (16x16)
// --------------------------------------------------------------------------------------

const FACE_PILOT: &[&str] = &[
    "....kkkkkkkk....",
    "...koooooooook..",
    "..koooooooooook.",
    "..kookkkkkkkook.",
    "..kokccccccckok.",
    "..kokcwcccccko..",
    "..kokcccccccko..",
    "..kookkkkkkkook.",
    "..kooonnnnnoook.",
    "...kknnnnnnnkk..",
    "....knnkknnk....",
    "....knnnnnnk....",
    "...kkdnnnndkk...",
    "..kttdddddddtk..",
    ".kttttttottttk..",
    ".kttttttttttttk.",
];

const FACE_CHIEF: &[&str] = &[
    "................",
    ".....kkkkkk.....",
    "....kllllllk....",
    "...klllllllllk..",
    "...knnnnnnnnnk..",
    "...knkknnkknnk..",
    "...knwknnkwknk..",
    "...knnnnnnnnnk..",
    "...knnnnbnnnnk..",
    "...kllllllllllk.",
    "...klllkkklllk..",
    "....kllllllllk..",
    ".....kkllllkk...",
    "...kbbbbnnbbbbk.",
    "..kbbbbbbbbbbbbk",
    "..kbbbybbbbybbbk",
];

const FACE_CONTROL: &[&str] = &[
    "................",
    ".....kkkkkk.....",
    "....kuuuuuuk....",
    "...kuuuuuuuuk...",
    "..kkuunnnnuukk..",
    "..kmknnnnnnkmk..",
    "..kmknknnknkmk..",
    "..kmknnnnnnkmk..",
    "..kmknnnnnnkmk..",
    "...k.knrrnk.km..",
    ".....knnnnk..m..",
    "......kkkk..kmk.",
    "....kddddddk....",
    "...kdcdddddddk..",
    "..kdddddddddddk.",
    "..kdddcdddddddk.",
];

const FACE_ENEMY: &[&str] = &[
    "................",
    "....kkkkkkkk....",
    "...kmmmmmmmmk...",
    "..kmmmmmmmmmmk..",
    "..kmkkkkkkkkmk..",
    "..kmkrrkkrrkmk..",
    "..kmkrrkkrrkmk..",
    "..kmkkkkkkkkmk..",
    "..kmmmmmmmmmmk..",
    "..kmkdkdkdkdmk..",
    "...kmmmmmmmmk...",
    "....kkmmmmkk....",
    "......kmmk......",
    "...kkkkmmkkkk...",
    "..kddddddddddk..",
    "..kddrddddrddk..",
];

const FACE_SYSTEM: &[&str] = &[
    "kkkkkkkkkkkkkkkk",
    "kmmmmmmmmmmmmmmk",
    "kmkkkkkkkkkkkkmk",
    "kmkggkgggkggkkmk",
    "kmkkkkkkkkkkkkmk",
    "kmkgkggkkgggkkmk",
    "kmkkkkkkkkkkkkmk",
    "kmkgggkgkggkkkmk",
    "kmkkkkkkkkkkkkmk",
    "kmkgkkkkkkkkkkmk",
    "kmkkkkkkkkkkkkmk",
    "kmmmmmmmmmmmmmmk",
    "kkkkkkkkkkkkkkkk",
    "....kmmmmmmk....",
    "..kkmmmmmmmmkk..",
    "..kkkkkkkkkkkk..",
];

// --------------------------------------------------------------------------------------
// Salvage icons (16x16), index matches `PartInfo::icon`
// --------------------------------------------------------------------------------------

const ICONS: &[&[&str]] = &[
    // 0 capacitor bank
    &[
        "................",
        "...kk......kk...",
        "...kk......kk...",
        "..kkkkkkkkkkkk..",
        "..kccckccckcck..",
        "..kcwckcwckcwk..",
        "..kccckccckcck..",
        "..kccckccckcck..",
        "..kccckccckcck..",
        "..kccckccckcck..",
        "..kccckccckcck..",
        "..kmmmkmmmkmmk..",
        "..kkkkkkkkkkkk..",
        "...kyk....kyk...",
        "...kyk....kyk...",
        "................",
    ],
    // 1 leg actuator
    &[
        "......kkkk......",
        ".....kbbbbk.....",
        ".....kbllbk.....",
        "......kbbk......",
        "......kmmk......",
        "......kmmk......",
        ".....kkllkk.....",
        ".....kloolk.....",
        ".....kloolk.....",
        ".....kkllkk.....",
        "......kmmk......",
        "......kmmk......",
        ".....kmmmmk.....",
        "....kbbbbbbk....",
        "...kbbbbbbbbk...",
        "...kkkkkkkkkk...",
    ],
    // 2 sensor dish
    &[
        "................",
        "..kkkk..........",
        ".ksssskk........",
        "ksssssssk.......",
        "kssttsssskk.....",
        ".kssttsssssk....",
        "..kssstssssk.r..",
        "...kssssssk.rr..",
        "....kssssk.r....",
        ".....kkkk.k.....",
        ".......kmmk.....",
        "........kmk.....",
        "........kmk.....",
        ".......kmmmk....",
        "......kkkkkkk...",
        "................",
    ],
    // 3 radiator
    &[
        "................",
        ".kkkkkkkkkkkkkk.",
        ".kclclclclclclk.",
        ".kclclclclclclk.",
        ".kclclclclclclk.",
        ".kclclclclclclk.",
        ".kclclclclclclk.",
        ".kclclclclclclk.",
        ".kclclclclclclk.",
        ".kkkkkkkkkkkkkk.",
        "....kmk..kmk....",
        "....kck..kck....",
        "....kck..kck....",
        "...kkck..kckk...",
        "...kcck..kcck...",
        "................",
    ],
    // 4 hive reactor sleeve
    &[
        "......kkkk......",
        "....kkppppkk....",
        "...kppqqqqppk...",
        "..kpqkkggkkqpk..",
        "..kpqkggggkqpk..",
        ".kpqkggwwggkqpk.",
        ".kpqkggwwggkqpk.",
        ".kpqkggggggkqpk.",
        "..kpqkggggkqpk..",
        "..kpqqkkkkqqpk..",
        "...kpqqqqqqpk...",
        "..k.kkppppkk.k..",
        ".k...k.kk.k...k.",
        "k...k..k...k...k",
        "...k...k....k...",
        "................",
    ],
    // 5 recoil dampers
    &[
        "................",
        "....kk....kk....",
        "...kllk..kllk...",
        "...kmmk..kmmk...",
        "...kkkk..kkkk...",
        "...kyyk..kyyk...",
        "...kkkk..kkkk...",
        "...kyyk..kyyk...",
        "...kkkk..kkkk...",
        "...kyyk..kyyk...",
        "...kkkk..kkkk...",
        "...kmmk..kmmk...",
        "...kmmk..kmmk...",
        "..kbbbbkkbbbbk..",
        "..kkkkkkkkkkkk..",
        "................",
    ],
    // 6 tram battery
    &[
        "................",
        "....kk....kk....",
        "...krrk..kmmk...",
        ".kkkkkkkkkkkkkk.",
        ".kyyyyyyyyyyyyk.",
        ".kykkkkkkkkkkyk.",
        ".kykggggggggkyk.",
        ".kykggggggggkyk.",
        ".kykggkkkkkkkyk.",
        ".kykkkkkkkkkkyk.",
        ".kyyyyyyyyyyyyk.",
        ".kykykykykykykk.",
        ".kyyyyyyyyyyyyk.",
        ".kkkkkkkkkkkkkk.",
        "..kk........kk..",
        "................",
    ],
    // 7 original spares crate
    &[
        "................",
        "................",
        "..kkkkkkkkkkkk..",
        ".kbbbbbbbbbbbbk.",
        ".kbuuuuuuuuuubk.",
        ".kkkkkkkkkkkkkk.",
        ".kbbbbbbbbbbbbk.",
        ".kbbkooookbbbbk.",
        ".kbbkokkokbbbbk.",
        ".kbbkooookbbbbk.",
        ".kbbbbbbbbbbbbk.",
        ".kbuubbbbbbuubk.",
        ".kbbbbbbbbbbbbk.",
        ".kkkkkkkkkkkkkk.",
        "................",
        "................",
    ],
    // 8 fuel rods
    &[
        "................",
        "...kk..kk..kk...",
        "..kmmkkmmkkmmk..",
        "..kggkkggkkggk..",
        "..kggkkggkkggk..",
        "..kgwkkgwkkgwk..",
        "..kggkkggkkggk..",
        "..kggkkggkkggk..",
        "..kggkkggkkggk..",
        "..kggkkggkkggk..",
        "..kggkkggkkggk..",
        "..kmmkkmmkkmmk..",
        "..kkkkkkkkkkkk..",
        ".kyykyykyykyyk..",
        ".kkkkkkkkkkkkk..",
        "................",
    ],
];

// --------------------------------------------------------------------------------------
// Hull diagram: a 40x46 grid where each pixel belongs to a section (or the frame).
// --------------------------------------------------------------------------------------

pub const HULL_W: usize = 40;
pub const HULL_H: usize = 46;

/// Section index (0..10) or 255 for structural frame, 254 for empty.
pub fn hull_map() -> Vec<u8> {
    use crate::sim::Section as S;
    let mut g = vec![254u8; HULL_W * HULL_H];
    let mut fill = |x0: usize, y0: usize, x1: usize, y1: usize, v: u8| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                g[y * HULL_W + x] = v;
            }
        }
    };
    let frame = 255u8;
    // Uplink mast + dish
    fill(19, 0, 20, 6, S::Uplink as u8);
    fill(16, 1, 23, 2, S::Uplink as u8);
    fill(17, 3, 22, 3, S::Uplink as u8);
    // Sensor head
    fill(14, 6, 25, 9, S::Sensors as u8);
    fill(12, 7, 13, 8, S::Sensors as u8);
    fill(26, 7, 27, 8, S::Sensors as u8);
    // Cockpit
    fill(15, 10, 24, 14, S::Cockpit as u8);
    // Shoulders / shield emitters
    fill(6, 12, 12, 17, S::ShieldGen as u8);
    fill(27, 12, 33, 17, S::ShieldGen as u8);
    // Torso frame
    fill(12, 15, 27, 27, frame);
    // Coolant pipes
    fill(13, 16, 14, 26, S::Coolant as u8);
    fill(25, 16, 26, 26, S::Coolant as u8);
    // Reactor core
    fill(16, 17, 23, 24, S::Reactor as u8);
    // Autocannon arm (left side of the picture)
    fill(3, 18, 10, 29, S::Autocannon as u8);
    fill(4, 30, 5, 35, S::Autocannon as u8);
    fill(8, 30, 9, 35, S::Autocannon as u8);
    // Railgun arm (right) with long barrel
    fill(29, 18, 36, 28, S::Railgun as u8);
    fill(32, 29, 33, 40, S::Railgun as u8);
    fill(31, 39, 34, 40, S::Railgun as u8);
    // Hips
    fill(14, 28, 25, 30, frame);
    // Legs
    fill(13, 31, 18, 42, S::LeftLeg as u8);
    fill(11, 43, 19, 45, S::LeftLeg as u8);
    fill(21, 31, 26, 42, S::RightLeg as u8);
    fill(20, 43, 28, 45, S::RightLeg as u8);
    g
}

// --------------------------------------------------------------------------------------
// Procedural backdrops
// --------------------------------------------------------------------------------------

fn put(img: &mut ColorImage, x: i32, y: i32, c: [u8; 3]) {
    let [w, h] = img.size;
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        img.pixels[y as usize * w + x as usize] = Color32::from_rgb(c[0], c[1], c[2]);
    }
}

fn lerp3(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}

/// Ordered-dither threshold for a pixel-art gradient look.
fn bayer(x: i32, y: i32) -> f32 {
    const M: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    (M[(y & 3) as usize][(x & 3) as usize] as f32 + 0.5) / 16.0
}

pub const PANO_W: usize = 1280;
pub const PANO_H: usize = 150;
pub const HORIZON: i32 = 104;

/// A 360-degree panorama of the dying city, sampled by the external camera.
pub fn city_panorama() -> ColorImage {
    let (w, h) = (PANO_W as i32, PANO_H as i32);
    let mut img = ColorImage::new([PANO_W, PANO_H], vec![Color32::BLACK; PANO_W * PANO_H]);
    let sky_top = [24, 18, 34];
    let sky_mid = [92, 44, 52];
    let sky_low = [196, 98, 54];
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / HORIZON as f32;
            let c = if t < 0.6 {
                let tt = t / 0.6;
                if tt + (bayer(x, y) - 0.5) * 0.18 < 0.5 { lerp3(sky_top, sky_mid, tt) } else { lerp3(sky_top, sky_mid, tt + 0.1) }
            } else {
                let tt = (t - 0.6) / 0.4;
                let q = ((tt * 5.0) + bayer(x, y) * 0.9).floor() / 5.0;
                lerp3(sky_mid, sky_low, q)
            };
            put(&mut img, x, y, c);
        }
    }
    // Stars in the upper sky
    for i in 0..220 {
        let x = (hash2(i, 1, 7) % PANO_W as u32) as i32;
        let y = (hash2(i, 2, 7) % 40) as i32;
        put(&mut img, x, y, [180, 170, 200]);
    }
    // A sickly moon
    for dy in -6..=6 {
        for dx in -6..=6 {
            if dx * dx + dy * dy <= 36 {
                let shade = if dx * dx + dy * dy > 25 { [190, 170, 140] } else { [232, 214, 170] };
                put(&mut img, 300 + dx, 26 + dy, shade);
            }
        }
    }
    // Smoke columns
    for col in 0..6 {
        let cx = (hash2(col, 9, 3) % PANO_W as u32) as i32;
        for y in 10..HORIZON {
            let spread = ((HORIZON - y) as f32 * 0.25) as i32 + 2;
            let drift = ((y as f32) * 0.12).sin() * 6.0 + (HORIZON - y) as f32 * 0.3;
            for dx in -spread..=spread {
                if hashf(cx + dx, y, 11) > 0.45 + (dx.abs() as f32 / spread as f32) * 0.5 {
                    put(&mut img, cx + dx + drift as i32, y, [48, 34, 40]);
                }
            }
        }
    }
    // Far skyline
    let far = [52, 34, 48];
    let mut x = 0;
    while x < w {
        let bw = 8 + (hash2(x, 3, 1) % 14) as i32;
        let bh = 14 + (hash2(x, 4, 1) % 34) as i32;
        let broken = hash2(x, 5, 1).is_multiple_of(3);
        for bx in x..(x + bw).min(w) {
            let mut top = HORIZON - bh;
            if broken {
                top += ((bx - x) as f32 * 0.9) as i32 % 9;
            }
            for y in top..HORIZON {
                put(&mut img, bx, y, far);
            }
        }
        x += bw + (hash2(x, 6, 1) % 4) as i32;
    }
    // Near skyline with windows and fires
    let near = [26, 20, 28];
    let mut x = 3;
    while x < w {
        let bw = 12 + (hash2(x, 7, 2) % 22) as i32;
        let bh = 20 + (hash2(x, 8, 2) % 46) as i32;
        let style = hash2(x, 9, 2) % 4;
        for bx in x..(x + bw).min(w) {
            let lx = bx - x;
            let mut top = HORIZON - bh;
            match style {
                0 => top += (lx * lx / 6).min(bh / 2),                     // collapsed corner
                1 => top += if (lx / 3) % 2 == 0 { 0 } else { 3 },         // jagged crown
                2 => top += ((lx as f32 * 0.7).sin().abs() * 6.0) as i32, // shelled
                _ => {}
            }
            for y in top..HORIZON {
                let mut c = near;
                if lx == 0 || lx == bw - 1 {
                    c = [18, 14, 20];
                }
                let wx = lx % 4;
                let wy = (y - top) % 5;
                if wx == 2 && wy == 2 && y < HORIZON - 4 && lx > 1 && lx < bw - 2 {
                    let r = hash2(bx, y, 13) % 23;
                    c = match r {
                        0 => [250, 196, 90],
                        1 => [230, 120, 50],
                        2..=4 => [60, 44, 56],
                        _ => [34, 26, 36],
                    };
                }
                put(&mut img, bx, y, c);
            }
            if hash2(bx, 1, 21).is_multiple_of(37) {
                // Fire on the roofline
                for fy in 0..(3 + (hash2(bx, 2, 21) % 5) as i32) {
                    let c = if fy < 2 { [250, 210, 90] } else { [230, 100, 40] };
                    put(&mut img, bx, top - fy, c);
                    put(&mut img, bx + 1, top - fy + 1, [200, 70, 30]);
                }
            }
        }
        // Antenna / crane silhouettes
        if style == 3 {
            for y in (HORIZON - bh - 10)..(HORIZON - bh) {
                put(&mut img, x + bw / 2, y, near);
            }
            put(&mut img, x + bw / 2, HORIZON - bh - 11, [255, 60, 40]);
        }
        x += bw + 6 + (hash2(x, 10, 2) % 18) as i32;
    }
    // Ground: cracked asphalt and rubble with perspective
    for y in HORIZON..h {
        let t = (y - HORIZON) as f32 / (h - HORIZON) as f32;
        for x in 0..w {
            let base = lerp3([40, 32, 34], [64, 52, 46], t + (bayer(x, y) - 0.5) * 0.25);
            let mut c = base;
            let n = hashf(x / 2, y, 31);
            if n > 0.93 {
                c = [86, 72, 60];
            } else if n < 0.05 {
                c = [24, 20, 22];
            }
            put(&mut img, x, y, c);
        }
    }
    // Rubble piles and burnt-out vehicles
    for i in 0..40 {
        let cx = (hash2(i, 40, 5) % PANO_W as u32) as i32;
        let cy = HORIZON + 4 + (hash2(i, 41, 5) % 36) as i32;
        let r = 3 + (hash2(i, 42, 5) % 6) as i32;
        for dy in -r..=0 {
            for dx in -r * 2..=r * 2 {
                if (dx * dx) / 4 + dy * dy <= r * r && hashf(cx + dx, cy + dy, 43) > 0.2 {
                    let c = if dy < -r / 2 { [96, 84, 72] } else { [58, 48, 44] };
                    put(&mut img, cx + dx, cy + dy, c);
                }
            }
        }
    }
    img
}

pub const HANGAR_W: usize = 320;
pub const HANGAR_H: usize = 180;

/// The ruined hangar: trusses, a broken skylight, and the mech standing in its cradle.
pub fn hangar_backdrop() -> ColorImage {
    let (w, h) = (HANGAR_W as i32, HANGAR_H as i32);
    let mut img = ColorImage::new([HANGAR_W, HANGAR_H], vec![Color32::BLACK; HANGAR_W * HANGAR_H]);
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / h as f32;
            let c = lerp3([14, 14, 20], [38, 34, 36], t + (bayer(x, y) - 0.5) * 0.12);
            put(&mut img, x, y, c);
        }
    }
    // Back wall panels
    for y in 30..140 {
        for x in 0..w {
            if x % 32 == 0 || y % 22 == 8 {
                put(&mut img, x, y, [22, 22, 28]);
            } else if hashf(x, y, 3) > 0.985 {
                put(&mut img, x, y, [60, 40, 30]);
            }
        }
    }
    // Roof trusses
    for x in 0..w {
        for y in [12, 13, 28, 29] {
            put(&mut img, x, y, [52, 50, 56]);
        }
        let m = x % 24;
        let y = 14 + if m < 12 { m * 14 / 12 } else { (24 - m) * 14 / 12 };
        put(&mut img, x, y.min(27), [44, 42, 48]);
    }
    // Broken skylight with light shafts
    for y in 0..12 {
        for x in 120..200 {
            if hashf(x / 3, y / 2, 5) > 0.35 {
                put(&mut img, x, y, [70, 76, 96]);
            }
        }
    }
    for y in 12..h {
        for x in 0..w {
            let center = 160.0 + (y as f32 - 12.0) * 0.35;
            let d = (x as f32 - center).abs();
            let half = 30.0 + (y as f32) * 0.18;
            if d < half && bayer(x, y) < 0.22 * (1.0 - d / half) + 0.04 {
                let i = img.pixels[(y * w + x) as usize];
                let c = lerp3([i.r(), i.g(), i.b()], [150, 150, 170], 0.25);
                put(&mut img, x, y, c);
            }
        }
    }
    // Floor
    for y in 140..h {
        for x in 0..w {
            let c = if (x + (y - 140) * 2) % 40 < 2 { [70, 60, 30] } else { lerp3([34, 30, 30], [52, 44, 40], (y - 140) as f32 / 40.0) };
            put(&mut img, x, y, c);
        }
        if y == 140 {
            for x in 0..w {
                put(&mut img, x, y, [74, 70, 70]);
            }
        }
    }
    // Hazard stripes on the floor edge
    for x in 0..w {
        for y in 170..174 {
            let c = if ((x + y) / 4) % 2 == 0 { [200, 160, 40] } else { [20, 20, 20] };
            put(&mut img, x, y, c);
        }
    }
    // Scaffolding gantries left and right
    for &gx in &[96i32, 222] {
        for y in 40..140 {
            put(&mut img, gx, y, [90, 80, 50]);
            put(&mut img, gx + 1, y, [60, 54, 36]);
            if y % 20 == 0 {
                for x in gx - 14..gx + 2 {
                    put(&mut img, x, y, [90, 80, 50]);
                }
            }
        }
    }
    // Crane hook
    for y in 30..58 {
        put(&mut img, 250, y, [70, 70, 76]);
    }
    for (dx, dy) in [(0, 58), (1, 59), (1, 60), (0, 61), (-1, 61), (-2, 60)] {
        put(&mut img, 250 + dx, dy, [180, 140, 40]);
    }
    // Junk piles on the floor
    for i in 0..18 {
        let cx = (hash2(i, 1, 77) % HANGAR_W as u32) as i32;
        if (110..210).contains(&cx) {
            continue;
        }
        let r = 3 + (hash2(i, 2, 77) % 5) as i32;
        for dy in -r..=0 {
            for dx in -r * 2..=r * 2 {
                if (dx * dx) / 4 + dy * dy <= r * r && hashf(cx + dx, dy, 78) > 0.25 {
                    let c = if hashf(cx + dx, dy, 79) > 0.8 { [140, 90, 50] } else { [64, 58, 56] };
                    put(&mut img, cx + dx, 150 + dy, c);
                }
            }
        }
    }
    img
}
