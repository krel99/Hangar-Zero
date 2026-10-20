//! Hand-drawn pixel art (as text grids) plus procedurally painted backdrops.
//! Small sprites are painted as rectangles every frame so they can flash and tint;
//! large backdrops are baked once into nearest-filtered egui textures.

use bevy_egui::egui::{self, Color32, ColorImage, Pos2, Rect, Vec2};

use crate::rng::{hash2, hashf};

/// Sentinel colour for `*` pixels: replaced by the accent passed to `paint_accent`.
const ACCENT: [u8; 3] = [255, 0, 255];

pub fn pal(c: char) -> Option<[u8; 3]> {
    Some(match c {
        // greys
        'k' => [13, 15, 18],
        'x' => [26, 28, 34],
        'd' => [40, 44, 52],
        'm' => [74, 82, 92],
        'l' => [124, 134, 146],
        'h' => [184, 194, 201],
        'w' => [244, 241, 224],
        // browns / rust / skin
        'u' => [70, 44, 30],
        'b' => [122, 74, 42],
        'z' => [150, 84, 46],
        'n' => [222, 170, 130],
        'N' => [240, 206, 172],
        // warm
        'R' => [120, 24, 22],
        'r' => [204, 48, 40],
        'o' => [228, 124, 40],
        'y' => [244, 202, 72],
        'Y' => [252, 240, 170],
        // olives / greens
        'e' => [44, 58, 38],
        't' => [104, 106, 58],
        's' => [150, 146, 90],
        'S' => [192, 188, 132],
        'G' => [36, 110, 60],
        'g' => [126, 240, 138],
        // cyan / blue
        'C' => [34, 96, 112],
        'c' => [95, 211, 230],
        'i' => [58, 78, 110],
        'I' => [124, 152, 200],
        // purples
        'q' => [80, 42, 128],
        'p' => [140, 76, 210],
        'P' => [196, 150, 240],
        '*' => ACCENT,
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
        let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
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
        self.paint_accent(p, min, scale, flash, tint, flip, None);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn paint_accent(&self, p: &egui::Painter, min: Pos2, scale: f32, flash: f32, tint: Option<Color32>, flip: bool, accent: Option<Color32>) {
        for y in 0..self.h {
            for x in 0..self.w {
                let sx = if flip { self.w - 1 - x } else { x };
                if let Some(mut c) = self.px[y * self.w + sx] {
                    if c == ACCENT {
                        let a = accent.unwrap_or(Color32::from_rgb(232, 168, 56));
                        c = [a.r(), a.g(), a.b()];
                    }
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
    /// Interior machinery per section, indexed by `Section::idx()`.
    pub props: Vec<Sprite>,
    pub arm_rail: Sprite,
    pub arm_ac: Sprite,
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
            props: PROPS.iter().map(|i| Sprite::parse(i)).collect(),
            arm_rail: Sprite::parse(ARM_RAIL),
            arm_ac: Sprite::parse(ARM_AC),
        }
    }
}

// --------------------------------------------------------------------------------------
// Enemies (all face left; the painter flips them when they orbit the other way)
// --------------------------------------------------------------------------------------

/// Jackal scout: a low four-legged runner with a chin cannon and one red eye.
const SCOUT: &[&str] = &[
    "........kk..............................",
    "........kck.............................",
    "...kkkkkkkk...........kkkk..............",
    "..kSSSSSSsk..........kddddk.............",
    ".kSsssssssskkkkkkkkkkkxxddk.............",
    ".ksskRRrrkssSSSSSSSSSSSSSssk............",
    ".ksskrrRRkssssssssssssssssssk...........",
    ".kssskkkkssstttsstttsstttsstk...........",
    ".kssssssssstttttttttttttttttk...........",
    "kkkkkkksssstttttttttttttttttek..........",
    "kmllllkssttttttttttttttttttteek.........",
    "kkkkkkkkttkkkkkkkkkkkkkkkkeeeek.........",
    "........kkkmlkdk........kmlkkkkkdk......",
    "..........kmlkdk.........kmlk..kdk......",
    ".........kmlkdk...........kmlk..kdk.....",
    "........kmlkdk.............kmlk..kdk....",
    ".......kmmkdk...............kmmk..kdk...",
    "......kmlkdk.................kmlk..kdk..",
    ".....kddkdk...................kddk..kdk.",
    "....kkkkkkk...................kkkkk.kkkk",
];

/// Mortar Crab: a wide braced carapace with hazard stripes and a mortar tube.
const ARTILLERY: &[&str] = &[
    "..........................................kkk...",
    ".........................................kllmk..",
    "........................................kllmk...",
    ".......................................kllmk....",
    "......................................kllmk.....",
    ".....................................kllmk......",
    "....................................kllmk.......",
    "...................................kllmk........",
    "..............kkkkkkkkkkkkkkkkkkkkkllmkk........",
    "...........kkkhhhhhhhhhhhhhhhhhhhllmkhhkk.......",
    ".........kkhhhllllllllllllllllllllmkllllhkk.....",
    "........khhlllllllllllllllllllllllklllllllhk....",
    ".......khlllllyykkyykkyykkyykkyykkyykklllllhk...",
    "......khlllllkkyykkyykkyykkyykkyykkyyklllllllk..",
    "......kllllllllllllllllllllllllllllllllllllllk..",
    "......klllmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmlllk..",
    "......kllmmmmmmmkkkkkkkkmmmmmmmmrRmmmmmmmmmmlk..",
    "......klmmmmmmmkddddddddkmmmmmmmmkkmmmmmmmmmmk..",
    "......kmmmmmmmkddddddddddkmmmmmmmmmmmmmmmmmmmk..",
    "......kkmmmmmmkdddddddddddkmmmmmmmmmmmmmmmmmkk..",
    ".......kkddddkkkkkkkkkkkkkkdddddddddddddddkk....",
    "........kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk....",
    ".....kmdk....kmdk......kmdk.....kmdk....kmdk....",
    "....kmdk.....kmdk.......kmdk.....kmdk....kmdk...",
    "...kmdk......kmdk........kmdk.....kmdk....kmdk..",
    "..kmdk.......kmdk.........kmdk.....kmdk....kmdk.",
    ".kkkk........kkkk.........kkkkk.....kkkk....kkkk",
];

/// Tick Carrier: a purple beetle-hive with glowing brood cells and a drone bay.
const CARRIER: &[&str] = &[
    "....k..........kkkkkkkkkk..........k....",
    ".....k.....kkkkPPPPPPPPPPkkkk.....k.....",
    "......k..kkPPPppppppppppppPPPkk..k......",
    ".......kkPPpppppppppppppppppppPkk.......",
    ".......kPppppkggkpppppppkggkppppk.......",
    "......kPpppppkgGgkpppppkgGgkppppPk......",
    "......kpppppppkggkpppppkggkppppppk......",
    ".....kppppkggkpppppkggkpppppkggkpppk....",
    ".....kppppkgGgkppppkgGgkppppkgGgkppk....",
    ".....kpppppkggkpppppkggkpppppkggkppk....",
    ".....kqpppppppppppppppppppppppppppqk....",
    ".....kqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqk...",
    "....kmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmk..",
    "....kllllllllllllllllllllllllllllllllk..",
    "....kmmmxxxmmmmxxxmmmmxxxmmmmxxxmmmmmk..",
    "....kkmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmkk..",
    ".....kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk...",
    "....kmdk..kmdk..kmdk....kmdk..kmdk..kmdk",
    "...kmdk...kmdk...kmdk....kmdk..kmdk..kmdk",
    "..kmdk....kmdk....kmdk....kmdk..kmdk..kmd",
    ".kmdk.....kmdk.....kmdk....kmdk..kmdk..km",
    "kkkk......kkkk.....kkkkk...kkkk..kkkk..kk",
];

/// Boarding drone in flight.
const DRONE: &[&str] = &[
    "k...kkk...k",
    ".k.kpppk.k.",
    "..kpqqqpk..",
    "kkpqgggqpkk",
    "..kpqgqpk..",
    "...kppk....",
    "..k.kk.k...",
];

/// Boarding drone clamped to the hull.
const TICK: &[&str] = &[
    ".k........k.",
    "..k..kk..k..",
    "...kkppkk...",
    "..kpPppPpk..",
    ".kppqggqppk.",
    "kpppqgGqpppk",
    ".kqpppppppk.",
    "..kkqqqqkk..",
    ".k.k.kk.k.k.",
    "k..k....k..k",
];

const PILOT: &[&str] = &[
    "...kkkk...",
    "..koooook.",
    ".kooooook.",
    ".kokcccck.",
    ".kokcCcck.",
    ".kooookk..",
    "..knnnk...",
    ".kttttkk..",
    "kttotttek.",
    "knttottnk.",
    "kkttottkk.",
    ".kttttttk.",
    ".kedkkdek.",
    ".kdk..kdk.",
    ".kdk..kdk.",
    "kkkk..kkkk",
];

const WRENCH: &[&str] = &[
    ".lhl.",
    "l.h.l",
    ".lhl.",
    "..h..",
    "..h..",
    "..h..",
    "..h..",
    "..l..",
];

// --------------------------------------------------------------------------------------
// Interior props, one per section (Reactor, Cockpit, Sensors, Coolant, LeftLeg, RightLeg,
// Railgun, Autocannon, ShieldGen, Uplink)
// --------------------------------------------------------------------------------------

const PROPS: &[&[&str]] = &[
    // Reactor: containment vessel with a glowing core and control rods
    &[
        "...kk.kk.kk...",
        "...kl.kl.kl...",
        ".kkkkkkkkkkkk.",
        "kmllllllllllmk",
        "kmkkkkkkkkkkmk",
        "kmkooyyyyookmk",
        "kmkoyYYYYyokmk",
        "kmkoyYYYYyokmk",
        "kmkooyyyyookmk",
        "kmkkkkkkkkkkmk",
        "kmmmmmmmmmmmmk",
        ".kkkkkkkkkkkk.",
    ],
    // Cockpit: the chair, two screens and a stick
    &[
        "kkkkkk..kkkkkk",
        "kgggGk..kGcCck",
        "kgGggk..kcccCk",
        "kkkkkk..kkkkkk",
        "..kk......kk..",
        "...kuuuk......",
        "...kuoukkk....",
        "...kuuuuuuk...",
        "..kmkuuuuuk...",
        "..kmkkkkkkk...",
        "...kmk..kmk...",
        "...kkk..kkk...",
    ],
    // Sensors: rack of scopes and a spinning dish motor
    &[
        "......kkkk....",
        ".....kSSssk...",
        "......kssk....",
        ".kkkkkkmmkkkk.",
        ".kmmmmmmmmmmk.",
        ".kkkkkkkkkkkk.",
        ".kkeegkkeegkk.",
        ".kkgeekkgeekk.",
        ".kkkkkkkkkkkk.",
        ".kmrmgmmmymmk.",
        ".kmmmmmmmmmmk.",
        ".kkkkkkkkkkkk.",
    ],
    // Coolant: tank, pipes and a wheel valve
    &[
        "..kkkkk.......",
        ".kcCCCck......",
        ".kcCCCckkkkkkk",
        ".kcCCCckllllmk",
        ".kcCCCck.kk...",
        ".kkkkkkk.kk...",
        "...kk...kllk..",
        "...kk..klrrlk.",
        "kkkkkkkklrrlk.",
        "kllllllkkllk..",
        "kkkkkkkk.kk...",
        "........kkkk..",
    ],
    // Left leg: hydraulic piston and gearbox
    &[
        "....kkkk......",
        "...kbbbbk.....",
        "...kbzzbk.....",
        "....khlk......",
        "....khlk......",
        "...kkllkk.....",
        "..kmhoolmk....",
        "..kmhoolmk....",
        "...kkllkk.....",
        "....khlk......",
        "..kbbbbbbk....",
        "..kkkkkkkk....",
    ],
    // Right leg: same actuator, mirrored
    &[
        "......kkkk....",
        ".....kbbbbk...",
        ".....kbzzbk...",
        "......khlk....",
        "......khlk....",
        ".....kkllkk...",
        "....kmhoolmk..",
        "....kmhoolmk..",
        ".....kkllkk...",
        "......khlk....",
        "....kbbbbbbk..",
        "....kkkkkkkk..",
    ],
    // Railgun: capacitor bank and the breech
    &[
        ".kkk.kkk.kkk..",
        ".kcck.kcck.kck",
        ".kcCk.kcCk.kCk",
        ".kcCk.kcCk.kCk",
        ".kkkk.kkkk.kkk",
        "kmmmmmmmmmmmmk",
        "kkkkkkkkkkkkkk",
        "kllllllllllllk",
        "kmxxxxxxxxxxmk",
        "kllllllllllllk",
        "kkkkkkkkkkkkkk",
        "..kk......kk..",
    ],
    // Autocannon: ammo drum with a belt feeding up
    &[
        "kkk...........",
        "kyykkk........",
        "kkkyykkk......",
        "..kkkyykkk....",
        "....kkkyykk...",
        ".kkkkkkkkkkkk.",
        "kmlllllllllmdk",
        "kmlkkkkkkkklmk",
        "kmlkyykyykklmk",
        "kmlkkkkkkkklmk",
        "kmmmmmmmmmmmmk",
        ".kkkkkkkkkkkk.",
    ],
    // Shield emitters: coil stack with arcing
    &[
        "..c.......c...",
        ".kkkkkkkkkkkk.",
        "kiIIIIIIIIIIik",
        "kkkkkkkkkkkkkk",
        "kiIIIIIIIIIIik",
        "kkkkkkkkkkkkkk",
        "kiIIIIIIIIIIik",
        "kkkkkkkkkkkkkk",
        "kiIIIIIIIIIIik",
        "kkkkkkkkkkkkkk",
        ".kmmmmmmmmmmk.",
        ".kkkkkkkkkkkk.",
    ],
    // Uplink: antenna base, transformer and a patch panel
    &[
        "......kk......",
        "......kck.....",
        "......kck.....",
        "....kkkkkkk...",
        "....kmmmmmk...",
        "kkkkkkkkkkkkkk",
        "kmmmmmmmmmmmmk",
        "kmgkrkgkykgkmk",
        "kmmmmmmmmmmmmk",
        "kmxxmxxmxxmxmk",
        "kmmmmmmmmmmmmk",
        "kkkkkkkkkkkkkk",
    ],
];

// --------------------------------------------------------------------------------------
// Cockpit framing: the mech's own arms in the corners of the camera feed.
// `*` pixels take the maker accent of the mounted weapon.
// --------------------------------------------------------------------------------------

const ARM_RAIL: &[&str] = &[
    "..........kkk...................................",
    ".........kdddk..................................",
    "........kddmddk.................................",
    ".......kddmlmddk................................",
    "......kddmllmddk................................",
    ".....kddmlllmddk................................",
    "....kddmllmmddk.................................",
    "...kddmllmmddkk.................................",
    "..kddmllmmddk.kkkk..............................",
    ".kddmllmmddk.kddddk.............................",
    "kddmllmmddk.kdd**ddk............................",
    "kdmllmmddk.kdd****ddk...........................",
    "kdlmmmddk.kdd******ddkkkkkkkkkkkkkkkkkkkkkkkkkkk",
    "kkmmdddk.kdddddddddddddddddddddddddddddddddddddd",
    "kkkkkkk.kddmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmm",
    "........kdmmllllllllllllllllllllllllllllllllllll",
    "........kdmlxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "........kdmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmm",
    "........kddmmmmmkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk",
    "........kdddddddkddddddddddddddddddddddddddddddd",
    "........kkdddddkkddddxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ".........kkkkkkkkddddddddddddddddddddddddddddddd",
    "................kddddddddddddddddddddddddddddddd",
    "................kdddddkkkkkkkkkkkkkkkkkkkkkkkkkk",
    "................kdddddkxxxxxxxxxxxxxxxxxxxxxxxxx",
    "................kdddddkkkkkkkkkkkkkkkkkkkkkkkkkk",
    "................kddddddddddddddddddddddddddddddd",
    "................kddddddddddddddddddddddddddddddd",
    "................kddddddddddddddddddddddddddddddd",
    "................kddddddddddddddddddddddddddddddd",
];

const ARM_AC: &[&str] = &[
    "..........kkkkkk................................",
    ".........kdmmmmdk...............................",
    "........kdmlxxlmdk..............................",
    ".......kdmlxxxxlmdk.............................",
    "......kdmlxxxxxxlmdk............................",
    ".....kdmmllxxxxllmdk............................",
    "....kddmmllxxxxllmdk............................",
    "...kddmmmllllllllmdk............................",
    "..kddmmmmmmmmmmmmddk.kkkk.......................",
    ".kddmmmmmmmmmmmmddk.kddddk......................",
    "kddmmmmmmmmmmmmddk.kdd**ddk.....................",
    "kdmmmmmmmmmmmmddk.kdd****ddk....................",
    "kdlmmmmmmmmmmddk.kdd******ddkkkkkkkkkkkkkkkkkkkk",
    "kkmmmmmmmmmdddk.kddddddddddddddddddddddddddddddd",
    "kkkkkkkkkkkkkk.kddmmmmmmmmmmmmmmmmmmmmmmmmmmmmmm",
    "...............kdmmllllllllllllllllllllllllllllll",
    "...............kdmlxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "...............kdmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmmm",
    "...............kddmmmmmkkkkkkkkkkkkkkkkkkkkkkkkkk",
    "...............kdddddddkdddddddddddddddddddddddd",
    "...............kkdddddkkddddxxxxxxxxxxxxxxxxxxxxx",
    "................kkkkkkkkdddddddddddddddddddddddd",
    ".......................kddddddddddddddddddddddddd",
    ".......................kdddddkkkkkkkkkkkkkkkkkkk",
    ".......................kdddddkxxxxxxxxxxxxxxxxxx",
    ".......................kdddddkkkkkkkkkkkkkkkkkkk",
    ".......................kddddddddddddddddddddddddd",
    ".......................kddddddddddddddddddddddddd",
    ".......................kddddddddddddddddddddddddd",
    ".......................kddddddddddddddddddddddddd",
];

// --------------------------------------------------------------------------------------
// Radio portraits (24x24)
// --------------------------------------------------------------------------------------

const FACE_PILOT: &[&str] = &[
    "........kkkkkkkk........",
    "......kkooooooookk......",
    ".....kooooooooooook.....",
    "....koooooooooooooook...",
    "...kooooooooooooooook...",
    "...koookkkkkkkkkkoook...",
    "...kookccccccccccckook..",
    "...kokcCcccccccccccckok.",
    "...kokccwccccccccccckok.",
    "...kokcccccccccccccckok.",
    "...kokccccccccccccCckok.",
    "...kookkkkkkkkkkkkkkook.",
    "...koonNNNNNNNNNNNnoook.",
    "...kkkNNNNNNNNNNNNNkkk..",
    ".....kNNkkNNNNkkNNNk....",
    ".....kNNkkNNNNkkNNNk....",
    ".....kNNNNNNNNNNNNNk....",
    ".....kNNNNNnNNnNNNNk....",
    "......kNNNNNNNNNNNk.....",
    "......kkNNNrrrrNNkk.....",
    "....kkttkkNNNNNNkkttkk..",
    "...kttttttkkkkkkttttttk.",
    "..kttttttttootttttttttk.",
    "..kttttttttttttttttttttk",
];

const FACE_CHIEF: &[&str] = &[
    "......kkkkkkkkkkkk......",
    ".....kuuuuuuuuuuuuk.....",
    "....kuuuuuuuuuuuuuuk....",
    "...kuuuuuuuuuuuuuuuuk...",
    "...kkkkkkkkkkkkkkkkkkkk.",
    "..kuuuuuuuuuuuuuuuuuuuk.",
    "...kkkkkkkkkkkkkkkkkkkk.",
    "....khhhhhhhhhhhhhhhk...",
    "....khnnnnnnnnnnnnnnk...",
    "....knnnnnnnnnnnnnnnk...",
    "....knnkknnnnnnkknnnk...",
    "....knkwkknnnnkwkknnk...",
    "....knnkknnnnnnkknnnk...",
    "....knnnnnnnbnnnnnnnk...",
    "....knnnnnnbbbnnnnnnk...",
    "....khhhhhhhhhhhhhhhk...",
    "....khhhhhkkkkkhhhhhk...",
    "....khhhhkrRRRkhhhhhk...",
    "....khhhhhkkkkkhhhhhk...",
    ".....khhhhhhhhhhhhhk....",
    "......khhhhhhhhhhhk.....",
    "...kkbbkkhhhhhhhkkbbkk..",
    "..kbbbbbbkkkkkkkbbbbbbk.",
    ".kbbbbbbbybbbbbbbbbbbbbk",
];

const FACE_CONTROL: &[&str] = &[
    ".......kkkkkkkkkk.......",
    ".....kkuuuuuuuuuukk.....",
    "....kuuuuuuuuuuuuuuk....",
    "...kuuuuuuuuuuuuuuuuk...",
    "..kuuuuuuuuuuuuuuuuuuk..",
    "..kuukkkkkkkkkkkkkkuuk..",
    ".kmkunNNNNNNNNNNNnukmk..",
    ".kmkunNNNNNNNNNNNnukmk..",
    ".kmkuNNNNNNNNNNNNNukmk..",
    ".kmkuNkkkkNNNNkkkkukmk..",
    ".kmkuNkwkkkNNkwkkkukmk..",
    ".kmkuNkkkkNNNNkkkkukmk..",
    ".kmkuNNNNNNNNNNNNNukmk..",
    ".kmkuNNNNNNNNNNNNNukmk..",
    ".kkkuNNNNNrrrNNNNNukkk..",
    "..kkuNNNNNNNNNNNNNukmk..",
    "....kuNNNNNNNNNNNuk.km..",
    ".....kuNNNNNNNNNukkkmk..",
    "......kkNNNNNNNkkmmmk...",
    ".......kkkNNNkkkkkk.....",
    "...kkiiikkkkkkkiiikk....",
    "..kiiiiiiiiikiiiiiiiik..",
    ".kiiiiiiiiiikiiiiiiiiik.",
    ".kiiiicIiiiiikiiiiiiiik.",
];

const FACE_ENEMY: &[&str] = &[
    "kdkdkdkdkdkdkdkdkdkdkdkd",
    "dkdkdkdkdkdkdkdkdkdkdkdk",
    ".....kkkkkkkkkkkkkk.....",
    "....kmmmmmmmmmmmmmmk....",
    "...kmmmmmmmmmmmmmmmmk...",
    "...kmmdddddddddddddmk...",
    "...kmdkkkkkkkkkkkkkdmk..",
    "...kmdkRrrkkkkRrrkdmk...",
    "...kmdkrrRkkkkrrRkdmk...",
    "...kmdkkkkkkkkkkkkkdmk..",
    "...kmmdddddddddddddmmk..",
    "...kmmmmmmmmmmmmmmmmmk..",
    "...kmmdxdxdxdxdxdxdmmk..",
    "...kmmdxdxdxdxdxdxdmmk..",
    "....kmmdddddddddddmmk...",
    ".....kmmmmmmmmmmmmmk....",
    "......kkmmmmmmmmmkk.....",
    "........kkkmmmkkk.......",
    "....kkkkkkkkmkkkkkkkk...",
    "...kddddddddddddddddk...",
    "..kddddrddddddddddrdddk.",
    "..kdddddddddddddddddddk.",
    "dkdkdkdkdkdkdkdkdkdkdkdk",
    "kdkdkdkdkdkdkdkdkdkdkdkd",
];

const FACE_SYSTEM: &[&str] = &[
    "kkkkkkkkkkkkkkkkkkkkkkkk",
    "kmmmmmmmmmmmmmmmmmmmmmmk",
    "kmkkkkkkkkkkkkkkkkkkkkmk",
    "kmkeeeeeeeeeeeeeeeeeekmk",
    "kmkeGGeeeeeeeeeeeeGGekmk",
    "kmkeGggGeeeeeeeeGggGekmk",
    "kmkeeGggGeeeeeeGggGeekmk",
    "kmkeeeGggGeeeeGggGeeekmk",
    "kmkeeeeGggGGGGggGeeeekmk",
    "kmkeeeeeGggggggGeeeeekmk",
    "kmkeeeeeeGgwwgGeeeeeekmk",
    "kmkeeeeeeGgwwgGeeeeeekmk",
    "kmkeeeeeGggggggGeeeeekmk",
    "kmkeeeeGggGGGGggGeeeekmk",
    "kmkeeeGggGeeeeGggGeeekmk",
    "kmkeeGggGeeeeeeGggGeekmk",
    "kmkeGggGeeeeeeeeGggGekmk",
    "kmkeGGeeeeeeeeeeeeGGekmk",
    "kmkeeeeeeeeeeeeeeeeeekmk",
    "kmkkkkkkkkkkkkkkkkkkkkmk",
    "kmmmmmmmmrmmmmmmgmmmmmmk",
    "kkkkkkkkkkkkkkkkkkkkkkkk",
    ".....kkmmmmmmmmmmkk.....",
    "...kkkkkkkkkkkkkkkkkk...",
];

// --------------------------------------------------------------------------------------
// Salvage icons (20x20), index matches `PartInfo::icon`
// --------------------------------------------------------------------------------------

const ICONS: &[&[&str]] = &[
    // 0 capacitor bank
    &[
        "....kk....kk....kk..",
        "....kyk...kyk...kyk.",
        "...kkkkk.kkkkk.kkkkk",
        "...khhhk.khhhk.khhhk",
        "...kcCck.kcCck.kcCck",
        "...kcCck.kcCck.kcCck",
        "...kwCck.kwCck.kwCck",
        "...kcCck.kcCck.kcCck",
        "...kcCck.kcCck.kcCck",
        "...kcCck.kcCck.kcCck",
        "...kcCck.kcCck.kcCck",
        "...kcCck.kcCck.kcCck",
        "...kmdmk.kmdmk.kmdmk",
        "...kkkkk.kkkkk.kkkkk",
        "..kmmmmmmmmmmmmmmmmk",
        "..kmlllllllllllllllk",
        "..kkkkkkkkkkkkkkkkkk",
        "....kyk....kyk......",
        "...kyk....kyk.......",
        "...kk.....kk........",
    ],
    // 1 leg actuator
    &[
        "........kkkk........",
        ".......kbbbbk.......",
        ".......kbzzbk.......",
        ".......kbbbbk.......",
        "........kllk........",
        "........khlk........",
        "........khlk........",
        ".......kkllkk.......",
        "......kmhhllmk......",
        "......kmhoolmk......",
        "......kmhoolmk......",
        "......kmhhllmk......",
        ".......kkllkk.......",
        "........khlk........",
        "........khlk........",
        "........kllk........",
        ".......kbbbbk.......",
        "......kbbzzbbk......",
        ".....kbbbbbbbbk.....",
        ".....kkkkkkkkkk.....",
    ],
    // 2 sensor dish
    &[
        "...kkkkk............",
        "..kSSSSSkk..........",
        ".kSSSsssssk.........",
        "kSSsssssssskk.......",
        "kSsttsssssssk.......",
        ".kssttsssssssk..r...",
        ".ksssttssssssk.r.r..",
        "..ksssttssssk..r.r..",
        "...kssssssskk...r...",
        "....kkssskk.........",
        "......kkkk..........",
        ".......kmmk.........",
        "........kmlk........",
        "........kmlk........",
        "........kmlk........",
        "........kmlk........",
        ".......kmmlmk.......",
        "......kmmmmmmk......",
        ".....kkkkkkkkkk.....",
        "....................",
    ],
    // 3 radiator
    &[
        "....................",
        ".kkkkkkkkkkkkkkkkkk.",
        ".kclclclclclclclclk.",
        ".kcIcIcIcIcIcIcIcIk.",
        ".kclclclclclclclclk.",
        ".kcIcIcIcIcIcIcIcIk.",
        ".kclclclclclclclclk.",
        ".kcIcIcIcIcIcIcIcIk.",
        ".kclclclclclclclclk.",
        ".kcIcIcIcIcIcIcIcIk.",
        ".kclclclclclclclclk.",
        ".kkkkkkkkkkkkkkkkkk.",
        "....kmk......kmk....",
        "....kck......kck....",
        "....kck......kck....",
        "...kkckk....kkckk...",
        "...kccck....kccck...",
        "...kcIck....kcIck...",
        "...kkkkk....kkkkk...",
        "....................",
    ],
    // 4 hive reactor sleeve
    &[
        ".......kkkkkk.......",
        ".....kkPPPPPPkk.....",
        "....kPPpppppppPk....",
        "...kPppqqqqqqpppk...",
        "..kPpqkkggggkkqppk..",
        "..kpqkgggGGgggkqpk..",
        ".kPpqkggGggGggkqppk.",
        ".kpqkggGgwwgGggkqpk.",
        ".kpqkggGgwwgGggkqpk.",
        ".kPpqkggGggGggkqppk.",
        "..kpqkgggGGgggkqpk..",
        "..kPpqkkggggkkqppk..",
        "...kpqqqqqqqqqqppk..",
        "...k.kPpppppppPk.k..",
        "..k...kkPPPPkk...k..",
        ".k...k..kkkk..k...k.",
        "k...k....k.....k...k",
        "...k.....k......k...",
        "..k......k.......k..",
        "....................",
    ],
    // 5 recoil dampers
    &[
        "....................",
        "....kkk......kkk....",
        "...khhlk....khhlk...",
        "...kllmk....kllmk...",
        "...kkkkk....kkkkk...",
        "...kyyyk....kyyyk...",
        "...kkkkk....kkkkk...",
        "...kyyyk....kyyyk...",
        "...kkkkk....kkkkk...",
        "...kyyyk....kyyyk...",
        "...kkkkk....kkkkk...",
        "...kyyyk....kyyyk...",
        "...kkkkk....kkkkk...",
        "...klmmk....klmmk...",
        "...klmmk....klmmk...",
        "..kkbbbkk..kkbbbkk..",
        "..kbbzbbk..kbbzbbk..",
        "..kkkkkkkkkkkkkkkkk.",
        "....................",
        "....................",
    ],
    // 6 tram battery
    &[
        "....................",
        ".....kk......kk.....",
        "....krrk....kmmk....",
        "..kkkkkkkkkkkkkkkk..",
        ".kyyyyyyyyyyyyyyyyk.",
        ".kyYkkkkkkkkkkkkYyk.",
        ".kyykggggggggggkyyk.",
        ".kyykgGGGGGGGGgkyyk.",
        ".kyykgggggggkkkkyyk.",
        ".kyykkkkkkkkkkkkyyk.",
        ".kyyyyyyyyyyyyyyyyk.",
        ".kykkkkkkkkkkkkkkyk.",
        ".kykwYYYYkkkkkkkkyk.",
        ".kykkkkkkkkkkkkkkyk.",
        ".kyyyyyyyyyyyyyyyyk.",
        ".kkkkkkkkkkkkkkkkkk.",
        "...kmk........kmk...",
        "...kkk........kkk...",
        "....................",
        "....................",
    ],
    // 7 original spares crate
    &[
        "....................",
        "....................",
        "..kkkkkkkkkkkkkkkk..",
        ".kbbbbbbbbbbbbbbbbk.",
        ".kbuuuuuuuuuuuuuubk.",
        ".kbuSSSSSSSSSSSSubk.",
        ".kbuuuuuuuuuuuuuubk.",
        ".kkkkkkkkkkkkkkkkkk.",
        ".kbbbbbbbbbbbbbbbbk.",
        ".kbbkoooookbbbbbbbk.",
        ".kbbkokkookbbkzzbbk.",
        ".kbbkoookokbbbkzbbk.",
        ".kbbkoooookbbbbbbbk.",
        ".kbbbbbbbbbbbbbbbbk.",
        ".kbzzbbbbbbbbbbzzbk.",
        ".kbbbbbbbbbbbbbbbbk.",
        ".kkkkkkkkkkkkkkkkkk.",
        "....................",
        "....................",
        "....................",
    ],
    // 8 fuel rods
    &[
        "....................",
        "...kk...kk...kk.....",
        "..kmmk.kmmk.kmmk....",
        "..kggk.kggk.kggk....",
        "..kgGk.kgGk.kgGk....",
        "..kgGk.kgGk.kgGk....",
        "..kwGk.kwGk.kwGk....",
        "..kgGk.kgGk.kgGk....",
        "..kgGk.kgGk.kgGk....",
        "..kgGk.kgGk.kgGk....",
        "..kgGk.kgGk.kgGk....",
        "..kgGk.kgGk.kgGk....",
        "..kmdk.kmdk.kmdk....",
        "..kkkk.kkkk.kkkk....",
        ".kyyykyyyykyyyyk....",
        ".kkkkkkkkkkkkkkkk...",
        "....k.k.k.k.k.......",
        ".....y.y.y.y........",
        "....................",
        "....................",
    ],
    // 9 CCTV cluster: two tram cameras on a bracket with a status lamp
    &[
        "....................",
        ".......kkkkk........",
        "......kmmmmmk.......",
        ".kkkkkkmdddmmkkkkk..",
        "kllllkmdkkkdmklllllk",
        "kddddkmdkcCkdmkddddk",
        "kkkkkkmdkkkdmkkkkkkk",
        "......kmdddmmk......",
        ".......kkkkk........",
        "........kmk.........",
        ".....kkkkmkkkk......",
        "....kmmmmmmmmmk.....",
        "...kmdddddddddmk....",
        "...kmdkkkkkkkdmk....",
        "...kmdkcCCCckdmk....",
        "...kmdkCCwCCkdmk....",
        "...kmdkcCCCckdmk....",
        "...kmdkkkkkkkdmk....",
        "...kkmmmgmmmmmkk....",
        "....kkkkkkkkkkk.....",
    ],
];

// --------------------------------------------------------------------------------------
// Hull diagram: a 40x46 grid where each pixel belongs to a section (or the frame),
// plus a detail overlay (rivets, seams, glass, lights) that both the damage diagram
// and the hangar portrait shade with.
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

pub const HD_NONE: u8 = 0;
pub const HD_HI: u8 = 1;
pub const HD_LO: u8 = 2;
pub const HD_RIVET: u8 = 3;
pub const HD_SEAM: u8 = 4;
pub const HD_GLASS: u8 = 5;
pub const HD_GLOW: u8 = 6;
pub const HD_LIGHT: u8 = 7;
pub const HD_VENT: u8 = 8;
pub const HD_HAZARD: u8 = 9;

/// Surface detail for the hull map, same layout.
pub fn hull_detail() -> Vec<u8> {
    struct Grid(Vec<u8>);
    impl Grid {
        fn set(&mut self, x: usize, y: usize, v: u8) {
            self.0[y * HULL_W + x] = v;
        }
        fn hline(&mut self, x0: usize, x1: usize, y: usize, v: u8) {
            for x in x0..=x1 {
                self.set(x, y, v);
            }
        }
    }
    let mut g = Grid(vec![HD_NONE; HULL_W * HULL_H]);
    // Uplink: dish ribs and a beacon on top
    g.hline(16, 23, 1, HD_HI);
    g.hline(17, 22, 3, HD_LO);
    g.set(19, 0, HD_LIGHT);
    g.set(20, 0, HD_LIGHT);
    for y in [2, 4, 6] {
        g.set(19, y, HD_SEAM);
    }
    // Sensor head: visor band with a red eye, antennae stubs
    g.hline(15, 24, 6, HD_HI);
    g.hline(15, 18, 8, HD_SEAM);
    g.hline(21, 24, 8, HD_SEAM);
    g.set(19, 8, HD_LIGHT);
    g.set(20, 8, HD_LIGHT);
    g.set(12, 7, HD_HI);
    g.set(27, 7, HD_HI);
    for x in [14, 25] {
        g.set(x, 9, HD_RIVET);
    }
    // Cockpit: canopy glass with a frame, rivets
    g.hline(16, 23, 11, HD_GLASS);
    g.hline(16, 23, 12, HD_GLASS);
    g.set(19, 11, HD_SEAM);
    g.set(19, 12, HD_SEAM);
    g.set(20, 11, HD_SEAM);
    g.set(20, 12, HD_SEAM);
    g.hline(15, 24, 10, HD_HI);
    g.hline(15, 24, 14, HD_LO);
    for x in [15, 24] {
        g.set(x, 13, HD_RIVET);
    }
    // Shield emitters: fin plates
    for x0 in [6usize, 27] {
        g.hline(x0, x0 + 6, 12, HD_HI);
        g.hline(x0, x0 + 6, 14, HD_SEAM);
        g.hline(x0, x0 + 6, 16, HD_SEAM);
        g.set(x0 + 3, 13, HD_LIGHT);
        g.set(x0 + 1, 17, HD_RIVET);
        g.set(x0 + 5, 17, HD_RIVET);
    }
    // Torso frame: grille and hazard stripes on the hips
    for y in (15..=27).step_by(2) {
        g.set(12, y, HD_VENT);
        g.set(27, y, HD_VENT);
    }
    g.hline(15, 24, 16, HD_SEAM);
    g.hline(15, 24, 25, HD_SEAM);
    g.hline(14, 25, 29, HD_HAZARD);
    // Coolant lines: pipe clamps and vents
    for y in (17..=25).step_by(3) {
        g.set(13, y, HD_LO);
        g.set(14, y, HD_HI);
        g.set(25, y, HD_HI);
        g.set(26, y, HD_LO);
    }
    for y in (18..=26).step_by(3) {
        g.set(13, y, HD_VENT);
        g.set(26, y, HD_VENT);
    }
    // Reactor: a ring of bolts around a glowing core
    for (x, y) in [(16, 17), (23, 17), (16, 24), (23, 24)] {
        g.set(x, y, HD_RIVET);
    }
    g.hline(17, 22, 18, HD_LO);
    g.hline(17, 22, 23, HD_LO);
    for y in 19..=22 {
        g.set(17, y, HD_LO);
        g.set(22, y, HD_LO);
    }
    for y in 19..=22 {
        for x in 18..=21 {
            g.set(x, y, HD_GLOW);
        }
    }
    // Autocannon arm: ammo drum, feed slot, twin barrels with vents
    g.hline(3, 10, 18, HD_HI);
    g.hline(4, 9, 21, HD_SEAM);
    g.hline(4, 9, 27, HD_SEAM);
    for y in 23..=25 {
        g.set(5, y, HD_LO);
        g.set(8, y, HD_LO);
    }
    for (x, y) in [(3, 19), (10, 19), (3, 28), (10, 28)] {
        g.set(x, y, HD_RIVET);
    }
    for y in (31..=35).step_by(2) {
        g.set(4, y, HD_VENT);
        g.set(8, y, HD_VENT);
    }
    g.set(5, 30, HD_HI);
    g.set(9, 30, HD_HI);
    // Railgun: capacitor housing with rails, long barrel with rings and a muzzle
    g.hline(29, 36, 18, HD_HI);
    g.hline(30, 35, 20, HD_SEAM);
    for y in 22..=26 {
        g.set(31, y, HD_GLOW);
        g.set(34, y, HD_GLOW);
    }
    for (x, y) in [(29, 19), (36, 19), (29, 27), (36, 27)] {
        g.set(x, y, HD_RIVET);
    }
    for y in (30..=38).step_by(3) {
        g.set(32, y, HD_LO);
        g.set(33, y, HD_LO);
    }
    for y in 29..=38 {
        if y % 3 != 0 {
            g.set(32, y, HD_HI);
        }
    }
    g.hline(31, 34, 39, HD_HI);
    g.hline(31, 34, 40, HD_LO);
    // Legs: hip bolts, knee joint, shin plate, toe lip
    for (x0, x1) in [(13usize, 18usize), (21, 26)] {
        g.set(x0, 31, HD_RIVET);
        g.set(x1, 31, HD_RIVET);
        g.hline(x0, x1, 35, HD_SEAM);
        g.hline(x0 + 1, x1 - 1, 36, HD_LO);
        g.hline(x0, x1, 37, HD_SEAM);
        for y in 32..=42 {
            g.set(x0 + 1, y, HD_HI);
        }
        g.hline(x0 - 1, x1 + 1, 43, HD_HI);
        g.set(x0 + 2, 40, HD_VENT);
        g.set(x0 + 2, 41, HD_VENT);
    }
    g.hline(11, 19, 45, HD_LO);
    g.hline(20, 28, 45, HD_LO);
    g.0
}

/// Apply a detail code to a section's base colour.
pub fn hull_shade(base: Color32, d: u8, t: f64) -> Color32 {
    let mix = |a: Color32, b: [u8; 3], f: f32| {
        Color32::from_rgb(
            (a.r() as f32 + (b[0] as f32 - a.r() as f32) * f) as u8,
            (a.g() as f32 + (b[1] as f32 - a.g() as f32) * f) as u8,
            (a.b() as f32 + (b[2] as f32 - a.b() as f32) * f) as u8,
        )
    };
    match d {
        HD_HI => base.gamma_multiply(1.35),
        HD_LO => base.gamma_multiply(0.55),
        HD_RIVET => mix(base, [20, 22, 26], 0.75),
        HD_SEAM => base.gamma_multiply(0.42),
        HD_GLASS => mix(base, [250, 214, 130], 0.8),
        HD_GLOW => {
            let pulse = ((t * 2.5).sin() * 0.5 + 0.5) as f32;
            mix(base, [255, 160, 60], 0.55 + 0.35 * pulse)
        }
        HD_LIGHT => {
            if (t * 2.0).fract() < 0.5 { Color32::from_rgb(255, 70, 50) } else { mix(base, [120, 30, 24], 0.7) }
        }
        HD_VENT => mix(base, [16, 16, 20], 0.8),
        HD_HAZARD => mix(base, [230, 180, 40], 0.7),
        _ => base,
    }
}

/// Hazard stripes alternate; call with the x coordinate.
pub fn hull_hazard_dark(x: i32, y: i32) -> bool {
    ((x + y) / 2) % 2 == 0
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

fn get(img: &ColorImage, x: i32, y: i32) -> Option<[u8; 3]> {
    let [w, h] = img.size;
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        let c = img.pixels[y as usize * w + x as usize];
        (c.a() > 0).then(|| [c.r(), c.g(), c.b()])
    } else {
        None
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
pub const PANO_H: usize = 176;
pub const HORIZON: i32 = 104;

/// Far layer of the 360° panorama: sky, moon, mountains, distant towers and smoke.
/// Scrolled at a fraction of the camera's bearing for parallax.
pub fn city_far() -> ColorImage {
    let (w, h) = (PANO_W as i32, PANO_H as i32);
    let mut img = ColorImage::new([PANO_W, PANO_H], vec![Color32::BLACK; PANO_W * PANO_H]);
    let sky_top = [18, 14, 30];
    let sky_mid = [96, 42, 56];
    let sky_low = [206, 104, 56];
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / HORIZON as f32;
            let c = if t < 0.55 {
                let tt = t / 0.55;
                let q = ((tt * 6.0) + bayer(x, y) * 0.9).floor() / 6.0;
                lerp3(sky_top, sky_mid, q)
            } else {
                let tt = (t - 0.55) / 0.45;
                let q = ((tt * 7.0) + bayer(x, y) * 0.9).floor() / 7.0;
                lerp3(sky_mid, sky_low, q)
            };
            put(&mut img, x, y, c);
        }
    }
    // Stars
    for i in 0..320 {
        let x = (hash2(i, 1, 7) % PANO_W as u32) as i32;
        let y = (hash2(i, 2, 7) % 46) as i32;
        let bright = hash2(i, 3, 7) % 4 == 0;
        put(&mut img, x, y, if bright { [220, 210, 230] } else { [150, 140, 170] });
        if bright {
            put(&mut img, x + 1, y, [110, 100, 130]);
        }
    }
    // Moon, cratered and sickly, with a halo
    let (mx, my) = (300, 28);
    for dy in -10..=10 {
        for dx in -10..=10 {
            let d2 = dx * dx + dy * dy;
            if d2 <= 100 {
                let mut c = if d2 > 72 { [176, 158, 126] } else { [230, 212, 168] };
                if hashf(dx, dy, 44) > 0.86 {
                    c = [190, 170, 130];
                }
                if (dx - 3) * (dx - 3) + (dy + 2) * (dy + 2) < 5 || (dx + 4) * (dx + 4) + (dy - 3) * (dy - 3) < 3 {
                    c = [168, 150, 118];
                }
                put(&mut img, mx + dx, my + dy, c);
            } else if d2 <= 196 && bayer(dx, dy) < 0.25 {
                put(&mut img, mx + dx, my + dy, [96, 62, 70]);
            }
        }
    }
    // Mountains, two ridges
    let ridge = |img: &mut ColorImage, seed: u32, base: i32, amp: f32, col: [u8; 3], top: [u8; 3]| {
        for x in 0..w {
            let fx = x as f32 / w as f32 * std::f32::consts::TAU;
            let hgt = (fx * 3.0 + seed as f32).sin() * 0.5 + (fx * 7.0 + seed as f32 * 1.7).sin() * 0.3 + (fx * 19.0).sin() * 0.2;
            let hgt = (hgt * amp + amp * 0.6) as i32 + ((hash2(x / 3, 0, seed) % 3) as i32);
            for y in (base - hgt)..HORIZON {
                let c = if y < base - hgt + 2 { top } else { col };
                put(img, x, y, c);
            }
        }
    };
    ridge(&mut img, 11, HORIZON - 10, 22.0, [60, 36, 58], [84, 52, 72]);
    ridge(&mut img, 29, HORIZON - 4, 14.0, [46, 28, 46], [62, 38, 58]);
    // Smoke columns
    for col in 0..7 {
        let cx = (hash2(col, 9, 3) % PANO_W as u32) as i32;
        for y in 8..HORIZON {
            let spread = ((HORIZON - y) as f32 * 0.28) as i32 + 2;
            let drift = ((y as f32) * 0.11).sin() * 7.0 + (HORIZON - y) as f32 * 0.35;
            for dx in -spread..=spread {
                let edge = dx.abs() as f32 / spread as f32;
                if hashf(cx + dx, y, 11) > 0.42 + edge * 0.5 {
                    let shade = if hashf(cx + dx, y, 12) > 0.7 { [62, 44, 50] } else { [44, 30, 38] };
                    put(&mut img, cx + dx + drift as i32, y, shade);
                }
            }
        }
    }
    // Far skyline: dense dead towers, some with a single lit floor
    let far = [48, 30, 46];
    let far_hi = [64, 42, 60];
    let mut x = 0;
    while x < w {
        let bw = 6 + (hash2(x, 3, 1) % 14) as i32;
        let bh = 16 + (hash2(x, 4, 1) % 40) as i32;
        let broken = hash2(x, 5, 1) % 3 == 0;
        let lit = hash2(x, 6, 1) % 5 == 0;
        for bx in x..(x + bw).min(w) {
            let mut top = HORIZON - bh;
            if broken {
                top += ((bx - x) as f32 * 0.9) as i32 % 9;
            }
            for y in top..HORIZON {
                let c = if bx == x { far_hi } else if lit && y == top + 6 && (bx - x) % 2 == 0 { [150, 110, 70] } else { far };
                put(&mut img, bx, y, c);
            }
        }
        if hash2(x, 7, 1) % 4 == 0 {
            // Radio mast with a blinking light baked as red
            let mx = x + bw / 2;
            for y in (HORIZON - bh - 12)..(HORIZON - bh) {
                put(&mut img, mx, y, far);
            }
            put(&mut img, mx, HORIZON - bh - 13, [230, 60, 40]);
        }
        x += bw + (hash2(x, 6, 1) % 4) as i32;
    }
    // The dead orbital tether: one colossal tower, visible from everywhere
    for y in 0..HORIZON {
        let hw = 2 + (y / 30);
        for dx in -hw..=hw {
            let c = if dx == -hw { [70, 46, 66] } else if (y / 6) % 2 == 0 && dx == 0 { [58, 36, 54] } else { [40, 26, 42] };
            put(&mut img, 900 + dx, y, c);
        }
        if y % 18 == 0 {
            put(&mut img, 900 - hw - 1, y, [255, 90, 60]);
            put(&mut img, 900 + hw + 1, y, [255, 90, 60]);
        }
    }
    img
}

/// Near layer: ruined buildings, cranes, pylons with wires, the street and its rubble.
/// Transparent above the roofline so the far layer shows through.
pub fn city_near() -> ColorImage {
    let (w, h) = (PANO_W as i32, PANO_H as i32);
    let mut img = ColorImage::new([PANO_W, PANO_H], vec![Color32::TRANSPARENT; PANO_W * PANO_H]);
    let wall = [26, 20, 28];
    let wall_lit = [40, 30, 40];
    let edge_c = [16, 12, 20];
    let mut x = 3;
    while x < w {
        let bw = 12 + (hash2(x, 7, 2) % 26) as i32;
        let bh = 22 + (hash2(x, 8, 2) % 50) as i32;
        let style = hash2(x, 9, 2) % 5;
        let win_w = 2 + (hash2(x, 10, 2) % 2) as i32;
        let pitch = 4 + (hash2(x, 11, 2) % 2) as i32;
        for bx in x..(x + bw).min(w) {
            let lx = bx - x;
            let mut top = HORIZON - bh;
            match style {
                0 => top += (lx * lx / 6).min(bh / 2),                      // collapsed corner
                1 => top += if (lx / 3) % 2 == 0 { 0 } else { 3 },          // crenellated
                2 => top += ((lx as f32 * 0.7).sin().abs() * 6.0) as i32,  // shelled
                3 => top += if lx > bw / 2 { (lx - bw / 2) * 2 } else { 0 }, // sheared
                _ => {}
            }
            // The face lit by the fires behind: right side a little warmer
            let face = if lx > bw * 2 / 3 { wall_lit } else { wall };
            for y in top..HORIZON {
                let mut c = face;
                if lx == 0 || lx == bw - 1 {
                    c = edge_c;
                }
                // Floor slabs
                if (y - top) % 7 == 6 && lx > 0 && lx < bw - 1 {
                    c = lerp3(c, [0, 0, 0], 0.35);
                }
                let wx = lx % pitch;
                let wy = (y - top) % 7;
                if wx >= 1 && wx <= win_w && (2..=4).contains(&wy) && y < HORIZON - 5 && lx > 1 && lx < bw - 2 {
                    let r = hash2(bx - wx, y - wy, 13) % 29;
                    c = match r {
                        0 => [250, 196, 90],
                        1 => [236, 128, 50],
                        2 => [120, 190, 210],
                        3..=5 => [62, 46, 58],
                        6..=8 => [16, 12, 18],
                        _ => [34, 26, 36],
                    };
                    if wy == 4 && r > 2 {
                        c = lerp3(c, [0, 0, 0], 0.4);
                    }
                }
                put(&mut img, bx, y, c);
            }
            // Fires on the roofline and in blown-out floors
            if hash2(bx, 1, 21) % 41 == 0 {
                for fy in 0..(3 + (hash2(bx, 2, 21) % 6) as i32) {
                    let c = if fy < 2 { [252, 220, 110] } else if fy < 4 { [236, 120, 46] } else { [170, 60, 30] };
                    put(&mut img, bx, top - fy, c);
                    put(&mut img, bx + 1, top - fy + 1, [210, 80, 34]);
                    put(&mut img, bx - 1, top - fy + 2, [120, 40, 30]);
                }
            }
        }
        // Rooftop clutter: water tanks, aerials, a crane
        let roof = HORIZON - bh;
        match hash2(x, 12, 2) % 4 {
            0 => {
                for y in (roof - 6)..roof {
                    for dx in 2..6 {
                        put(&mut img, x + dx, y, if y == roof - 6 { [52, 40, 48] } else { [30, 22, 30] });
                    }
                }
            }
            1 => {
                let mx = x + bw / 2;
                for y in (roof - 10)..roof {
                    put(&mut img, mx, y, wall);
                    if y % 3 == 0 {
                        put(&mut img, mx - 1, y, wall);
                        put(&mut img, mx + 1, y, wall);
                    }
                }
                put(&mut img, mx, roof - 11, [255, 60, 40]);
            }
            2 if bw > 20 => {
                // Tower crane
                let cx = x + 4;
                for y in (roof - 28)..roof {
                    put(&mut img, cx, y, [34, 28, 30]);
                    put(&mut img, cx + 1, y, [22, 18, 22]);
                    if y % 4 == 0 {
                        put(&mut img, cx + 2, y, [34, 28, 30]);
                    }
                }
                for dx in -6..=22 {
                    put(&mut img, cx + dx, roof - 28, [34, 28, 30]);
                    if dx % 3 == 0 {
                        put(&mut img, cx + dx, roof - 27, [22, 18, 22]);
                    }
                }
                for y in (roof - 27)..(roof - 14) {
                    put(&mut img, cx + 18, y, [22, 18, 22]);
                }
                put(&mut img, cx + 18, roof - 14, [120, 90, 30]);
                put(&mut img, cx + 18, roof - 13, [120, 90, 30]);
            }
            _ => {}
        }
        x += bw + 6 + (hash2(x, 10, 2) % 20) as i32;
    }
    // Ground: cracked asphalt with perspective bands
    for y in HORIZON..h {
        let t = (y - HORIZON) as f32 / (h - HORIZON) as f32;
        for x in 0..w {
            let base = lerp3([38, 30, 34], [66, 54, 48], t + (bayer(x, y) - 0.5) * 0.25);
            let mut c = base;
            let n = hashf(x / 2, y, 31);
            if n > 0.94 {
                c = [88, 74, 62];
            } else if n < 0.04 {
                c = [22, 18, 20];
            }
            // Cracks
            if hashf(x / 5 + y / 3, 0, 37) > 0.93 && hashf(x, y, 38) > 0.4 {
                c = [20, 16, 18];
            }
            put(&mut img, x, y, c);
        }
    }
    // Kerb line
    for x in 0..w {
        put(&mut img, x, HORIZON + 6, [52, 44, 40]);
        put(&mut img, x, HORIZON + 7, [30, 24, 26]);
    }
    // Power pylons with sagging wires
    let mut px = 60;
    while px < w {
        let top = HORIZON - 42;
        for y in top..HORIZON + 4 {
            put(&mut img, px, y, [30, 26, 30]);
            put(&mut img, px + 1, y, [22, 18, 22]);
            if (y - top) % 6 == 0 && y < HORIZON - 4 {
                for dx in -4..=5 {
                    put(&mut img, px + dx, y, [30, 26, 30]);
                }
            }
        }
        let next = px + 130;
        if next < w {
            for i in 0..=130 {
                let f = i as f32 / 130.0;
                let sag = (f * (1.0 - f)) * 26.0;
                for (arm, off) in [(0, -4), (6, 5)] {
                    let y = top + arm + sag as i32;
                    put(&mut img, px + off + i, y, [18, 14, 18]);
                }
            }
        }
        px = next;
    }
    // Street level: lamp posts, wrecked cars, barricades, rubble
    for i in 0..60 {
        let cx = (hash2(i, 40, 5) % PANO_W as u32) as i32;
        let depth = (hash2(i, 41, 5) % 40) as i32;
        let cy = HORIZON + 4 + depth;
        let kind = hash2(i, 42, 5) % 5;
        let k = 1 + depth / 20;
        match kind {
            0 | 1 => {
                // rubble pile
                let r = (3 + (hash2(i, 43, 5) % 6) as i32) * k;
                for dy in -r..=0 {
                    for dx in -r * 2..=r * 2 {
                        if (dx * dx) / 4 + dy * dy <= r * r && hashf(cx + dx, cy + dy, 43) > 0.2 {
                            let c = if dy < -r / 2 { [98, 86, 74] } else if hashf(cx + dx, dy, 45) > 0.85 { [140, 90, 50] } else { [58, 48, 44] };
                            put(&mut img, cx + dx, cy + dy, c);
                        }
                    }
                }
            }
            2 => {
                // burnt-out car
                let cw = 9 * k;
                let ch = 4 * k;
                for dy in -ch..=0 {
                    for dx in 0..cw {
                        let roof = dy < -ch / 2 && dx > cw / 5 && dx < cw * 4 / 5;
                        let body = dy >= -ch / 2;
                        if roof || body {
                            let c = if dy == -ch / 2 { [70, 60, 60] } else if hashf(dx, dy, i as u32) > 0.8 { [120, 70, 40] } else { [46, 40, 44] };
                            put(&mut img, cx + dx, cy + dy, c);
                        }
                    }
                }
                put(&mut img, cx + 1, cy + 1, [20, 18, 20]);
                put(&mut img, cx + cw - 2, cy + 1, [20, 18, 20]);
            }
            3 => {
                // lamp post, a few still lit
                let lit = hash2(i, 44, 5) % 3 == 0;
                let hgt = 18 * k;
                for y in (cy - hgt)..cy {
                    put(&mut img, cx, y, [40, 36, 40]);
                }
                put(&mut img, cx + 1, cy - hgt, [40, 36, 40]);
                put(&mut img, cx + 2, cy - hgt, [40, 36, 40]);
                put(&mut img, cx + 2, cy - hgt + 1, if lit { [255, 230, 150] } else { [60, 56, 60] });
                if lit {
                    for dy in 2..6 {
                        for dx in -dy..=dy {
                            if bayer(cx + dx, cy - hgt + dy) < 0.3 {
                                if let Some(c) = get(&img, cx + 2 + dx, cy - hgt + 1 + dy) {
                                    put(&mut img, cx + 2 + dx, cy - hgt + 1 + dy, lerp3(c, [255, 220, 140], 0.35));
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // concrete barricade with hazard paint
                for dy in -3 * k..=0 {
                    for dx in 0..7 * k {
                        let stripe = ((dx + dy) / (2 * k)) % 2 == 0 && dy > -2 * k;
                        put(&mut img, cx + dx, cy + dy, if stripe { [170, 130, 40] } else { [86, 80, 76] });
                    }
                }
            }
        }
    }
    // Puddles reflecting the sky
    for i in 0..18 {
        let cx = (hash2(i, 50, 5) % PANO_W as u32) as i32;
        let cy = HORIZON + 14 + (hash2(i, 51, 5) % 28) as i32;
        let r = 3 + (hash2(i, 52, 5) % 5) as i32;
        for dy in -1..=1 {
            for dx in -r * 2..=r * 2 {
                if (dx * dx) / 4 + dy * dy * 4 <= r * r {
                    put(&mut img, cx + dx, cy + dy, if bayer(dx, dy) < 0.5 { [118, 66, 60] } else { [70, 44, 52] });
                }
            }
        }
    }
    img
}

pub const HANGAR_W: usize = 400;
pub const HANGAR_H: usize = 225;

/// The ruined hangar: trusses, pipes, a broken skylight, gantries and junk.
pub fn hangar_backdrop() -> ColorImage {
    let (w, h) = (HANGAR_W as i32, HANGAR_H as i32);
    let mut img = ColorImage::new([HANGAR_W, HANGAR_H], vec![Color32::BLACK; HANGAR_W * HANGAR_H]);
    let floor_y = 176;
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / h as f32;
            let c = lerp3([12, 12, 18], [36, 32, 34], t + (bayer(x, y) - 0.5) * 0.12);
            put(&mut img, x, y, c);
        }
    }
    // Back wall: corrugated panels, stains, a painted bay number
    for y in 36..floor_y {
        for x in 0..w {
            let mut c = get(&img, x, y).unwrap();
            if x % 40 == 0 || x % 40 == 1 {
                c = [20, 20, 26];
            } else if y % 26 == 10 {
                c = [22, 22, 28];
            } else if (x / 4) % 2 == 0 {
                c = lerp3(c, [0, 0, 0], 0.12);
            }
            if hashf(x, y, 3) > 0.988 {
                c = [62, 42, 30];
            }
            // Rust streaks running down from the trusses
            if hashf(x, 0, 4) > 0.9 && y < 36 + (hashf(x, 1, 4) * 60.0) as i32 && hashf(x, y, 5) > 0.3 {
                c = lerp3(c, [96, 58, 36], 0.5);
            }
            put(&mut img, x, y, c);
        }
    }
    // Painted bay number and warning line
    let glyph_9: [&str; 7] = ["xxxx", "x..x", "x..x", "xxxx", "...x", "...x", "xxxx"];
    for (gy, row) in glyph_9.iter().enumerate() {
        for (gx, ch) in row.chars().enumerate() {
            if ch == 'x' {
                for dy in 0..5 {
                    for dx in 0..5 {
                        put(&mut img, 40 + gx as i32 * 5 + dx, 60 + gy as i32 * 5 + dy, [120, 100, 50]);
                    }
                }
            }
        }
    }
    for x in 20..90 {
        put(&mut img, x, 100, [110, 90, 44]);
        put(&mut img, x, 101, [80, 64, 30]);
    }
    // Pipes along the wall with valves
    for (py, col) in [(44, [66, 58, 50]), (48, [48, 44, 40]), (128, [60, 48, 44])] {
        for x in 0..w {
            put(&mut img, x, py, col);
            put(&mut img, x, py + 1, lerp3(col, [0, 0, 0], 0.4));
            if x % 60 == 30 {
                for dy in -2..=3 {
                    put(&mut img, x, py + dy, lerp3(col, [255, 255, 255], 0.2));
                    put(&mut img, x + 1, py + dy, lerp3(col, [0, 0, 0], 0.3));
                }
            }
            if x % 120 == 75 {
                for dy in -3..=0 {
                    put(&mut img, x, py + dy, [200, 60, 40]);
                    put(&mut img, x + 1, py + dy, [140, 40, 30]);
                }
            }
        }
    }
    // Conduit dropping to a fuse box
    for y in 50..96 {
        put(&mut img, 300, y, [48, 44, 40]);
    }
    for y in 96..116 {
        for x in 294..308 {
            put(&mut img, x, y, if x == 294 || y == 96 { [70, 66, 60] } else { [40, 38, 36] });
        }
    }
    put(&mut img, 304, 100, [120, 240, 130]);
    put(&mut img, 298, 100, [230, 60, 40]);
    // Roof trusses
    for x in 0..w {
        for y in [14, 15, 32, 33] {
            put(&mut img, x, y, [56, 54, 60]);
        }
        let m = x % 28;
        let y = 16 + if m < 14 { m * 16 / 14 } else { (28 - m) * 16 / 14 };
        put(&mut img, x, y.min(31), [46, 44, 50]);
        put(&mut img, x, (y + 1).min(31), [30, 28, 34]);
        if x % 28 == 0 {
            for y in 16..32 {
                put(&mut img, x, y, [46, 44, 50]);
            }
        }
    }
    // Floodlights hanging from the trusses, two dead, one working with a cone
    for (lx, on) in [(70, false), (200, true), (330, false)] {
        for y in 34..42 {
            put(&mut img, lx, y, [40, 38, 40]);
        }
        for dx in -5..=5 {
            for dy in 0..4 {
                let c = if dy == 3 { if on { [255, 240, 200] } else { [50, 48, 50] } } else { [60, 58, 60] };
                put(&mut img, lx + dx, 42 + dy, c);
            }
        }
        if on {
            for y in 46..floor_y + 20 {
                let half = 6 + (y - 46) * 2 / 5;
                for dx in -half..=half {
                    let f = 1.0 - dx.abs() as f32 / half as f32;
                    if bayer(lx + dx, y) < 0.3 * f * (1.0 - (y - 46) as f32 / 200.0) {
                        if let Some(c) = get(&img, lx + dx, y) {
                            put(&mut img, lx + dx, y, lerp3(c, [200, 190, 160], 0.28));
                        }
                    }
                }
            }
        }
    }
    // Broken skylight with light shafts
    for y in 0..14 {
        for x in 150..250 {
            if hashf(x / 3, y / 2, 5) > 0.3 {
                put(&mut img, x, y, [74, 80, 100]);
            }
            if x % 25 == 0 || y == 13 {
                put(&mut img, x, y, [40, 42, 52]);
            }
        }
    }
    for y in 14..h {
        for x in 0..w {
            let center = 200.0 + (y as f32 - 14.0) * 0.32;
            let d = (x as f32 - center).abs();
            let half = 36.0 + (y as f32) * 0.2;
            if d < half && bayer(x, y) < 0.2 * (1.0 - d / half) + 0.03 {
                let c = get(&img, x, y).unwrap();
                put(&mut img, x, y, lerp3(c, [150, 150, 172], 0.25));
            }
        }
    }
    // Floor: plates, grating strip, oil stains, painted lines
    for y in floor_y..h {
        for x in 0..w {
            let t = (y - floor_y) as f32 / (h - floor_y) as f32;
            let mut c = lerp3([36, 32, 32], [54, 46, 42], t);
            if (x + (y - floor_y) * 2) % 48 < 2 {
                c = [72, 62, 32];
            }
            if (y - floor_y) % 12 == 11 {
                c = lerp3(c, [0, 0, 0], 0.4);
            }
            if hashf(x / 3, y / 3, 61) > 0.96 {
                c = lerp3(c, [10, 8, 10], 0.5);
            }
            put(&mut img, x, y, c);
        }
    }
    for x in 0..w {
        put(&mut img, x, floor_y, [84, 80, 78]);
        put(&mut img, x, floor_y + 1, [40, 36, 36]);
    }
    for x in 0..w {
        for y in (h - 14)..(h - 10) {
            let c = if ((x + y) / 4) % 2 == 0 { [204, 164, 44] } else { [22, 22, 22] };
            put(&mut img, x, y, c);
        }
    }
    // Scaffolding gantries with railings and a catwalk
    for &gx in &[118i32, 282] {
        for y in 50..floor_y {
            put(&mut img, gx, y, [96, 84, 52]);
            put(&mut img, gx + 1, y, [64, 56, 36]);
            put(&mut img, gx + 14, y, [96, 84, 52]);
            put(&mut img, gx + 15, y, [64, 56, 36]);
            if y % 24 == 0 {
                for x in gx - 18..gx + 16 {
                    put(&mut img, x, y, [96, 84, 52]);
                    put(&mut img, x, y + 1, [50, 44, 30]);
                }
                for x in (gx - 18..gx + 16).step_by(4) {
                    for dy in -6..0 {
                        put(&mut img, x, y + dy, [80, 70, 44]);
                    }
                }
                for x in gx - 18..gx + 16 {
                    put(&mut img, x, y - 6, [80, 70, 44]);
                }
            }
            // Cross braces
            let m = (y - 50) % 24;
            put(&mut img, gx + 2 + m * 12 / 24, y, [70, 60, 40]);
            put(&mut img, gx + 13 - m * 12 / 24, y, [70, 60, 40]);
        }
    }
    // Overhead crane rail with hook and chains
    for x in 100..300 {
        put(&mut img, x, 34, [80, 78, 84]);
        put(&mut img, x, 35, [44, 42, 48]);
    }
    for y in 36..70 {
        put(&mut img, 262, y, if y % 2 == 0 { [90, 88, 92] } else { [60, 58, 64] });
        put(&mut img, 263, y, [40, 40, 44]);
    }
    for (dx, dy) in [(0, 70), (1, 71), (2, 72), (2, 73), (1, 74), (0, 75), (-1, 75), (-2, 74), (-2, 73)] {
        put(&mut img, 262 + dx, dy, [200, 160, 40]);
    }
    for y in 36..60 {
        put(&mut img, 140, y, if y % 2 == 0 { [70, 68, 72] } else { [48, 46, 50] });
    }
    // Junk: barrels, crates, a tool chest, tyres
    for i in 0..24 {
        let cx = (hash2(i, 1, 77) % HANGAR_W as u32) as i32;
        if (130..270).contains(&cx) {
            continue;
        }
        match hash2(i, 2, 77) % 4 {
            0 => {
                // barrel
                for dy in 0..12 {
                    for dx in 0..8 {
                        let mut c = if dy == 0 || dy == 11 { [70, 66, 60] } else if dx == 0 { [80, 70, 50] } else if dx == 7 { [40, 34, 28] } else { [110, 90, 40] };
                        if dy == 3 || dy == 8 {
                            c = [60, 54, 40];
                        }
                        if hashf(cx + dx, dy, 78) > 0.85 {
                            c = [96, 58, 36];
                        }
                        put(&mut img, cx + dx, floor_y - 12 + dy + (hash2(i, 3, 77) % 10) as i32, c);
                    }
                }
            }
            1 => {
                // crate
                let s = 8 + (hash2(i, 3, 77) % 6) as i32;
                for dy in 0..s {
                    for dx in 0..s {
                        let c = if dy == 0 || dx == 0 { [120, 84, 50] } else if dy == s - 1 || dx == s - 1 { [50, 34, 20] } else if dx == dy || dx == s - 1 - dy { [96, 66, 40] } else { [86, 58, 34] };
                        put(&mut img, cx + dx, floor_y - s + dy + 2, c);
                    }
                }
            }
            2 => {
                // tool chest with drawers
                for dy in 0..14 {
                    for dx in 0..18 {
                        let c = if dy == 0 { [200, 70, 50] } else if dy % 4 == 3 { [60, 20, 16] } else if dx == 16 && dy % 4 == 1 { [220, 200, 160] } else { [160, 50, 40] };
                        put(&mut img, cx + dx, floor_y - 14 + dy + 3, c);
                    }
                }
            }
            _ => {
                // stacked tyres
                for t in 0..3 {
                    for dx in 0..10 {
                        let c = if dx == 0 || dx == 9 { [20, 20, 22] } else if dx == 4 || dx == 5 { [44, 44, 48] } else { [30, 30, 34] };
                        for dy in 0..3 {
                            put(&mut img, cx + dx, floor_y - 2 - t * 3 - dy + 2, c);
                        }
                    }
                }
            }
        }
    }
    // Welding cables on the floor
    for x in 0..w {
        let y = floor_y + 24 + ((x as f32 * 0.05).sin() * 4.0) as i32 + ((x as f32 * 0.013).sin() * 6.0) as i32;
        put(&mut img, x, y, [22, 20, 22]);
        put(&mut img, x, y + 1, [40, 36, 38]);
    }
    img
}
