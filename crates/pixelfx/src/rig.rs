//! A fixed-length 3D skeleton rendered into flat pixel tones and one outline.
//! Ported from pixelfx core/rig.js and poses.js. Pose coordinates use y down;
//! the solver uses y up internally. Body dimensions and light direction are configurable.
use crate::{
    color::{Color, hex},
    math::{PI, round},
    painter::Painter,
};
type V = [f64; 3];
type M = [f64; 9];
fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V, s: f64) -> V {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn len(a: V) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn unit(a: V) -> V {
    let l = len(a);
    mul(a, 1.0 / if l == 0.0 { 1e-6 } else { l })
}
fn ry(t: f64) -> M {
    let (c, s) = (t.cos(), t.sin());
    [c, 0.0, s, 0.0, 1.0, 0.0, -s, 0.0, c]
}
fn rz(t: f64) -> M {
    let (c, s) = (t.cos(), t.sin());
    [c, -s, 0.0, s, c, 0.0, 0.0, 0.0, 1.0]
}
fn mm(a: M, b: M) -> M {
    let mut o = [0.0; 9];
    for r in 0..3 {
        for c in 0..3 {
            o[r * 3 + c] = a[r * 3] * b[c] + a[r * 3 + 1] * b[3 + c] + a[r * 3 + 2] * b[6 + c];
        }
    }
    o
}
fn mv(m: M, v: V) -> V {
    [
        m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
        m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
        m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
    ]
}
fn tv(m: M, v: V) -> V {
    [
        m[0] * v[0] + m[3] * v[1] + m[6] * v[2],
        m[1] * v[0] + m[4] * v[1] + m[7] * v[2],
        m[2] * v[0] + m[5] * v[1] + m[8] * v[2],
    ]
}
const DEG: f64 = PI / 180.0;
/// Weapons supported by the stock rig.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weapon {
    Sword,
    Greatsword,
    Dagger,
    Spear,
    Hammer,
    Scythe,
    Claw,
    Bow,
    Pistol,
    Launcher,
    Handle,
    Coupe,
    /// A champagne bottle as long as its wielder is tall, held by the neck: Pixelvania's Jeroboam.
    /// Not in the JavaScript original.
    Bottle,
    /// A tall candelabrum swung as a polearm, three lit candles at its head: Pixelvania's third weapon.
    /// Not in the JavaScript original.
    Candelabrum,
}
impl Weapon {
    fn two(self) -> f64 {
        match self {
            Self::Sword => -4.4,
            Self::Greatsword => -4.8,
            Self::Spear => -12.0,
            Self::Hammer => -7.0,
            Self::Scythe => -11.0,
            Self::Launcher => -9.0,
            Self::Bottle => -5.0,
            Self::Candelabrum => -10.0,
            _ => 0.0,
        }
    }
    fn across(self) -> bool {
        matches!(self, Self::Bow | Self::Pistol)
    }
}
/// Hands, feet and body attitude in screen coordinates relative to the feet.
/// Optional depths and chest turn follow the stock solver when absent.
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub lean: f64,
    pub crouch: f64,
    pub yaw: f64,
    pub fx: f64,
    pub fy: f64,
    pub bx: f64,
    pub by: f64,
    pub ffx: f64,
    pub ffy: f64,
    pub fbx: f64,
    pub fby: f64,
    pub hx: f64,
    pub hy: f64,
    pub twist: f64,
    pub sa: f64,
    pub sb: f64,
    pub wl: f64,
    pub wz: f64,
    pub turn: Option<f64>,
    pub fz: Option<f64>,
    pub bz: Option<f64>,
    pub fzz: Option<f64>,
    pub bzz: Option<f64>,
    pub weapon: Option<Weapon>,
    pub off_weapon: Option<Weapon>,
    pub two: bool,
    pub arms_front: bool,
    pub toe_f: bool,
    pub toe_b: bool,
    pub pull: bool,
    pub nocked: bool,
    pub face: i32,
    pub ghost: f64,
}
impl Default for Pose {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            lean: 0.0,
            crouch: 0.0,
            yaw: 0.0,
            fx: -3.0,
            fy: -15.0,
            bx: 5.0,
            by: -15.0,
            ffx: -2.0,
            ffy: -4.0,
            fbx: 3.0,
            fby: -4.0,
            hx: 0.0,
            hy: 0.0,
            twist: 0.0,
            sa: 0.0,
            sb: PI,
            wl: 1.0,
            wz: 1.0,
            turn: None,
            fz: None,
            bz: None,
            fzz: None,
            bzz: None,
            weapon: None,
            off_weapon: None,
            two: false,
            arms_front: false,
            toe_f: false,
            toe_b: false,
            pull: false,
            nocked: false,
            face: 1,
            ghost: 0.0,
        }
    }
}
impl Pose {
    pub fn ready() -> Self {
        Self {
            crouch: 3.0,
            fx: 11.0,
            fy: -21.0,
            bx: 9.0,
            by: -21.0,
            ffx: 7.0,
            fbx: -7.0,
            ..Self::default()
        }
    }
}
/// Bone lengths and offsets in art pixels.
#[derive(Clone, Debug)]
pub struct Measurements {
    pub pelvis: f64,
    pub spine1: f64,
    pub spine2: f64,
    pub shoulder_up: f64,
    pub shoulder_out: f64,
    pub neck_up: f64,
    pub head_up: f64,
    pub hip_down: f64,
    pub hip_out: f64,
    pub thigh: f64,
    pub shin: f64,
    pub upper: f64,
    pub fore: f64,
    pub turn: f64,
}
impl Default for Measurements {
    fn default() -> Self {
        Self {
            pelvis: 19.7,
            spine1: 2.8,
            spine2: 4.6,
            shoulder_up: 2.5,
            shoulder_out: 5.9,
            neck_up: 4.0,
            head_up: 3.1,
            hip_down: 1.1,
            hip_out: 2.8,
            thigh: 8.6,
            shin: 8.3,
            upper: 7.6,
            fore: 7.4,
            turn: 36.0 * DEG,
        }
    }
}
#[derive(Clone, Copy, Default)]
struct Basis {
    t: V,
    r: M,
}
#[derive(Clone, Copy)]
struct Limb {
    joint: V,
    end: V,
    v: V,
}
fn ik(a: V, target: V, l1: f64, l2: f64, pole: V) -> Limb {
    let mut to = sub(target, a);
    let mut d = len(to);
    let reach = l1 + l2 - 0.02;
    if d > reach {
        to = mul(to, reach / d);
        d = reach;
    }
    let u = unit(to);
    let along = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d.max(1e-12));
    let h = (l1 * l1 - along * along).max(0.0).sqrt();
    let v = sub(pole, mul(u, dot(pole, u)));
    let v = if len(v) < 1e-4 {
        unit(cross(u, [0.0, 0.0, 1.0]))
    } else {
        unit(v)
    };
    Limb {
        joint: add(a, add(mul(u, along), mul(v, h))),
        end: add(a, to),
        v,
    }
}
fn bone(a: V, b: V, pole: V) -> Basis {
    let x = unit(sub(b, a));
    let y = sub(pole, mul(x, dot(pole, x)));
    let y = if len(y) < 1e-4 {
        unit(cross(x, [0.0, 0.0, 1.0]))
    } else {
        unit(y)
    };
    let z = cross(x, y);
    Basis {
        t: a,
        r: [x[0], y[0], z[0], x[1], y[1], z[1], x[2], y[2], z[2]],
    }
}
struct Solved {
    b: [Basis; 15],
    arms: [Limb; 2],
    legs: [Limb; 2],
    shoulders: [V; 2],
    hips: [V; 2],
    axis: V,
    side: V,
}
fn solve(q: &Pose, m: &Measurements) -> Solved {
    let sa = q.sa;
    let axis_z = (1.0 - q.wl * q.wl).max(0.0).sqrt() * q.wz;
    let axis = [sa.cos() * q.wl, -sa.sin() * q.wl, axis_z];
    let side = [-sa.sin(), -sa.cos(), 0.0];
    let two = q.two && q.weapon.is_some();
    let near_z = q.fz.unwrap_or(if two { 3.0 } else { 4.5 });
    let fwd = (q.fx + 3.0) / 24.0;
    let far_fwd = if two { 0.0 } else { (q.bx - 5.0) / 22.0 };
    let mut turn = m.turn
        + if fwd >= 0.0 {
            -44.0 * DEG * fwd.min(1.0)
        } else {
            28.0 * DEG * (-fwd * 1.8).min(1.0)
        };
    turn += if far_fwd > 0.0 {
        34.0 * DEG * far_fwd.min(1.0)
    } else {
        0.0
    } + q.twist * 4.0 * DEG;
    turn = q.turn.unwrap_or(turn);
    let hip_turn = m.turn + (turn - m.turn) * 0.45;
    let tilt = q.lean.atan2(13.0);
    let rp = mm(rz(-tilt * 0.25), ry(-hip_turn));
    let rc = mm(rz(-tilt), ry(-turn));
    let ankles = [
        [q.ffx, -q.ffy - 1.8, q.fzz.unwrap_or(2.4)],
        [q.fbx, -q.fby - 1.8, q.bzz.unwrap_or(-2.4)],
    ];
    let mut pelvis = [q.lean * 0.2, m.pelvis - q.crouch, 0.0];
    let hip_at = |pelvis: V, s: f64| add(pelvis, mv(rp, [0.0, -m.hip_down, s * m.hip_out]));
    let max_leg = (m.thigh + m.shin) * 0.995;
    for (i, a) in ankles.iter().enumerate() {
        let h = hip_at(pelvis, if i == 0 { 1.0 } else { -1.0 });
        let dx = h[0] - a[0];
        let dz = h[2] - a[2];
        let flat = dx * dx + dz * dz;
        if flat < max_leg * max_leg {
            let top = a[1] + (max_leg * max_leg - flat).sqrt();
            if h[1] > top {
                pelvis[1] -= h[1] - top;
            }
        }
    }
    let waist = add(
        pelvis,
        [
            (tilt * 0.45).sin() * m.spine1,
            (tilt * 0.45).cos() * m.spine1,
            0.0,
        ],
    );
    let chest = add(waist, [tilt.sin() * m.spine2, tilt.cos() * m.spine2, 0.0]);
    let neck = add(chest, mv(rc, [0.0, m.neck_up, 0.0]));
    let head_turn = m.turn * 0.9 + (turn - m.turn) * 0.35;
    let nod = tilt * 0.35 + q.hy * 7.0 * DEG;
    let roll = q.hx * 7.0 * DEG;
    let rh = mm(rz(-nod - roll), ry(-head_turn));
    let head = add(neck, mv(rh, [0.15, m.head_up, 0.0]));
    let pick = |s: f64, target: V| {
        let base = add(chest, mv(rc, [0.0, m.shoulder_up, s * m.shoulder_out]));
        let to = sub(target, base);
        add(
            base,
            mul(unit(to), ((len(to) - 11.0).max(0.0) * 0.35).min(1.8)),
        )
    };
    let near_t = [q.fx, -q.fy, near_z];
    let far_t = if two {
        add(near_t, mul(axis, q.weapon.map_or(0.0, Weapon::two)))
    } else {
        [
            q.bx,
            -q.by,
            q.bz.unwrap_or(if q.arms_front { 5.5 } else { -4.5 }),
        ]
    };
    let sn = pick(1.0, near_t);
    let sf = pick(-1.0, far_t);
    let pole = |s: f64, t: V, shoulder: V, hold: Option<V>| {
        let up = ((t[1] - shoulder[1] + 2.0) / 10.0).clamp(0.0, 1.0);
        let base = [-0.55 + up * 1.1, -1.0 + up * 0.9, s * (0.45 + up * 0.5)];
        let Some(hold) = hold else {
            return base;
        };
        let u = unit(sub(t, shoulder));
        let (mut best, mut score) = (base, f64::INFINITY);
        for i in 0..16 {
            let a = i as f64 / 16.0 * PI * 2.0;
            let x = unit(cross(u, [0.3, 1.0, 0.2]));
            let y = cross(u, x);
            let v = add(mul(x, a.cos()), mul(y, a.sin()));
            let arm = ik(shoulder, t, m.upper, m.fore, v);
            let f = unit(sub(arm.end, arm.joint));
            let cost = dot(f, hold).abs() * 0.7 + (1.0 - dot(unit(base), v)) * 0.55;
            if cost < score {
                score = cost;
                best = v;
            }
        }
        best
    };
    let hold_n = q.weapon.map(|w| if w.across() { side } else { axis });
    let hold_f = if two {
        hold_n
    } else {
        q.off_weapon.map(|_| [q.sb.cos(), -q.sb.sin(), 0.0])
    };
    let an = ik(sn, near_t, m.upper, m.fore, pole(1.0, near_t, sn, hold_n));
    let af = ik(sf, far_t, m.upper, m.fore, pole(-1.0, far_t, sf, hold_f));
    let hn = hip_at(pelvis, 1.0);
    let hf = hip_at(pelvis, -1.0);
    let ln = ik(hn, ankles[0], m.thigh, m.shin, mv(rp, [1.0, 0.0, 0.35]));
    let lf = ik(hf, ankles[1], m.thigh, m.shin, mv(rp, [1.0, 0.0, -0.35]));
    let foot = |s: f64, end: V, toe: bool| {
        let t = hip_turn * 0.6 + s * 8.0 * DEG;
        Basis {
            t: end,
            r: mm(
                ry(-t.sin().atan2(t.cos())),
                rz(if toe { -55.0 * DEG } else { 0.0 }),
            ),
        }
    };
    let b = [
        Basis { t: pelvis, r: rp },
        Basis {
            t: waist,
            r: mm(rz(-tilt * 0.6), ry(-(turn + hip_turn) / 2.0)),
        },
        Basis { t: chest, r: rc },
        Basis { t: head, r: rh },
        Basis { t: neck, r: rh },
        bone(sn, an.joint, an.v),
        bone(an.joint, an.end, an.v),
        bone(sf, af.joint, af.v),
        bone(af.joint, af.end, af.v),
        bone(hn, ln.joint, ln.v),
        bone(ln.joint, ln.end, ln.v),
        bone(hf, lf.joint, lf.v),
        bone(lf.joint, lf.end, lf.v),
        foot(1.0, ln.end, q.toe_f),
        foot(-1.0, lf.end, q.toe_b),
    ];
    Solved {
        b,
        arms: [an, af],
        legs: [ln, lf],
        shoulders: [sn, sf],
        hips: [hn, hf],
        axis,
        side,
    }
}
impl Solved {
    fn turn(&mut self, a: f64) {
        let r = ry(-a);
        for f in &mut self.b {
            f.t = mv(r, f.t);
            f.r = mm(r, f.r);
        }
        for a in self.arms.iter_mut().chain(self.legs.iter_mut()) {
            a.joint = mv(r, a.joint);
            a.end = mv(r, a.end);
        }
        for v in self.shoulders.iter_mut().chain(self.hips.iter_mut()) {
            *v = mv(r, *v);
        }
        self.axis = mv(r, self.axis);
        self.side = mv(r, self.side);
    }
}
const BW: usize = 300;
const BH: usize = 240;
const OX: i32 = 150;
const OY: i32 = 180;
const INK: Color = hex(0x0e0c14);
const SEAM: Color = hex(0x2a2b33);
/// A rig pixel with its source in the 24 × 38 standing sprite, or -1 for weapons.
#[derive(Clone, Debug)]
pub struct RigPixel {
    pub x: i16,
    pub y: i16,
    pub color: Color,
    pub source: i16,
    pub ink: bool,
}
#[derive(Clone, Debug)]
pub struct RenderedPose {
    pub pixels: Vec<RigPixel>,
}
impl RenderedPose {
    pub fn draw(&self, p: &mut Painter<'_>) {
        for px in &self.pixels {
            p.px(px.x as f64, px.y as f64, px.color);
        }
    }
}
/// Headwear attached to the head bone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hat {
    #[default]
    None,
    TopHat,
    Plume,
    Feathers,
}

/// Pale carnival masks attached to the head bone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mask {
    #[default]
    None,
    Domino,
    Beak,
}

/// Hair silhouettes attached to the head bone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hair {
    #[default]
    None,
    Bouffant,
    Tower,
    Long,
}

/// Character proportions, body palette and lighting, independent of Bevy.
#[derive(Clone, Debug)]
pub struct Rig {
    pub measurements: Measurements,
    pub light: [f64; 3],
    pub shell: [Color; 4],
    pub core: [Color; 4],
    /// Length in pixels of a tailcoat's two tails hanging behind the hips; 0 for none.
    /// Not in the JavaScript original.
    pub tails: f64,
    /// Flared skirt length; zero omits it.
    pub skirt: f64,
    /// Length of two trailing silk ribbons.
    pub ribbons: f64,
    /// Radius of a belly pushed out in front of the waist; zero omits it.
    /// Not in the JavaScript original.
    pub belly: f64,
    // Costume parts are not in the JavaScript original.
    pub hat: Hat,
    pub mask: Mask,
    pub hair: Hair,
    /// Light-to-dark cloth and hair ramp.
    pub costume: [Color; 4],
    /// Light-to-dark porcelain and gilt ramp for masks and hat accents.
    pub trim: [Color; 4],
}
impl Default for Rig {
    fn default() -> Self {
        Self {
            measurements: Measurements::default(),
            light: [-0.55, 0.72, 0.5],
            shell: [hex(0xe4e6ec), hex(0xb6b9c3), hex(0x868995), hex(0x5c5e6a)],
            core: [hex(0x6e707c), hex(0x4b4d58), hex(0x34353f), hex(0x34353f)],
            tails: 0.0,
            skirt: 0.0,
            ribbons: 0.0,
            belly: 0.0,
            hat: Hat::None,
            mask: Mask::None,
            hair: Hair::None,
            costume: [hex(0x615344), hex(0x3d302b), hex(0x281f22), hex(0x17131b)],
            trim: [hex(0xf0dfb5), hex(0xcbb587), hex(0xa28a63), hex(0x796344)],
        }
    }
}
struct Raster {
    z: Vec<f32>,
    tone: Vec<f32>,
    part: Vec<u8>,
    mat: Vec<u8>,
    fr: Vec<i8>,
    color: Vec<Color>,
    touched: Vec<usize>,
    frame: i8,
    clip: bool,
    light: V,
}
impl Raster {
    fn new(light: V) -> Self {
        Self {
            z: vec![0.0; BW * BH],
            tone: vec![0.0; BW * BH],
            part: vec![0; BW * BH],
            mat: vec![0; BW * BH],
            fr: vec![0; BW * BH],
            color: vec![[0; 4]; BW * BH],
            touched: vec![],
            frame: 0,
            clip: false,
            light: unit(light),
        }
    }
    fn idx(x: i32, y: i32) -> Option<usize> {
        let x = OX + x;
        let y = OY - y;
        if x < 1 || x >= BW as i32 - 1 || y < 1 || y >= BH as i32 - 1 {
            None
        } else {
            Some(y as usize * BW + x as usize)
        }
    }
    fn plot(&mut self, x: i32, y: i32, z: f64, tone: f64, part: u8, mat: u8, c: Color) {
        let Some(i) = Self::idx(x, y) else {
            return;
        };
        if self.part[i] == 0 {
            self.touched.push(i);
        } else if z <= self.z[i] as f64 {
            return;
        }
        self.z[i] = z as f32;
        self.tone[i] = tone as f32;
        self.part[i] = part;
        self.mat[i] = mat;
        self.fr[i] = self.frame;
        self.color[i] = c;
    }
    fn sphere(&mut self, c: V, r: f64, part: u8, mat: u8) {
        for y in (c[1] - r).floor() as i32..=(c[1] + r).ceil() as i32 {
            for x in (c[0] - r).floor() as i32..=(c[0] + r).ceil() as i32 {
                let dx = x as f64 - c[0];
                let dy = y as f64 - c[1];
                let d2 = dx * dx + dy * dy;
                if d2 > r * r || (self.clip && (y as f64) < 1.5) {
                    continue;
                }
                let dz = (r * r - d2).sqrt();
                self.plot(
                    x,
                    y,
                    c[2] + dz,
                    dot([dx / r, dy / r, dz / r], self.light),
                    part,
                    mat,
                    [0; 4],
                );
            }
        }
    }
    fn sweep(&mut self, a: V, b: V, ra: f64, rb: f64, part: u8, mat: u8, bulge: f64) {
        let n = (len(sub(b, a)) / 0.4).ceil().max(2.0) as usize;
        for k in 0..=n {
            let u = k as f64 / n as f64;
            self.sphere(
                add(a, mul(sub(b, a), u)),
                ra + (rb - ra) * u + bulge * (u * PI).sin(),
                part,
                mat,
            );
        }
    }
    fn ellipsoid(&mut self, c: V, rmat: M, r: V, part: u8, mat: u8) {
        let big = r[0].max(r[1]).max(r[2]);
        let d = [
            1.0 / (r[0] * r[0]),
            1.0 / (r[1] * r[1]),
            1.0 / (r[2] * r[2]),
        ];
        let mut a = [0.0; 9];
        for i in 0..3 {
            for j in 0..3 {
                a[i * 3 + j] = rmat[i * 3] * d[0] * rmat[j * 3]
                    + rmat[i * 3 + 1] * d[1] * rmat[j * 3 + 1]
                    + rmat[i * 3 + 2] * d[2] * rmat[j * 3 + 2];
            }
        }
        for y in (c[1] - big).floor() as i32..=(c[1] + big).ceil() as i32 {
            for x in (c[0] - big).floor() as i32..=(c[0] + big).ceil() as i32 {
                if self.clip && (y as f64) < 1.5 {
                    continue;
                }
                let dx = x as f64 - c[0];
                let dy = y as f64 - c[1];
                let qa = a[8];
                let qb = 2.0 * (a[2] * dx + a[5] * dy);
                let qc = a[0] * dx * dx + 2.0 * a[1] * dx * dy + a[4] * dy * dy - 1.0;
                let disc = qb * qb - 4.0 * qa * qc;
                if disc < 0.0 {
                    continue;
                }
                let dz = (-qb + disc.sqrt()) / (2.0 * qa);
                let n = unit([
                    a[0] * dx + a[1] * dy + a[2] * dz,
                    a[3] * dx + a[4] * dy + a[5] * dz,
                    a[6] * dx + a[7] * dy + a[8] * dz,
                ]);
                self.plot(x, y, c[2] + dz, dot(n, self.light), part, mat, [0; 4]);
            }
        }
    }
    /// A tailcoat's two tails: from the back of the waist, down and out behind the legs,
    /// in the darker core material so they read as cloth over the figure.
    fn tails(&mut self, s: &Solved, length: f64) {
        let b = &s.b;
        self.frame = 0;
        for side in [-1.4, 1.4] {
            let top = add(b[0].t, mv(b[0].r, [-2.2, 2.4, side]));
            let tip = add(b[0].t, mv(b[0].r, [-6.5, 2.4 - length, side * 1.3]));
            self.sweep(top, tip, 2.4, 1.3, 25, 1, 0.4);
        }
    }
    /// Costume geometry uses head-local x forward, y up and z across the face.
    fn costume(&mut self, s: &Solved, rig: &Rig) {
        let head = s.b[3];
        let k = rig.measurements.head_up / Measurements::default().head_up;
        let at = |p: V| add(head.t, mv(head.r, mul(p, k)));
        self.frame = 3;
        // Parts 1–22 are body, 23–24 weapons and 25 tails. Materials 3–4 are cloth and trim.
        match rig.hair {
            Hair::None => {}
            Hair::Tower => {
                self.ellipsoid(at([-2.0, 5.0, 0.0]), head.r, mul([4.0, 7.0, 4.5], k), 26, 4);
                self.ellipsoid(
                    at([-2.0, 11.0, 0.0]),
                    head.r,
                    mul([3.0, 4.0, 3.5], k),
                    26,
                    4,
                );
            }
            Hair::Bouffant => {
                self.ellipsoid(at([-3.0, 5.0, 0.0]), head.r, mul([6.0, 6.0, 5.5], k), 26, 3);
                self.ellipsoid(at([-3.0, 0.0, 0.0]), head.r, mul([3.5, 3.5, 3.6], k), 26, 3);
            }
            Hair::Long => {
                self.ellipsoid(at([-1.8, 1.0, 0.0]), head.r, mul([3.4, 3.5, 3.4], k), 26, 3);
                for side in [-2.0, 2.0] {
                    self.sweep(
                        at([-3.0, 0.0, side]),
                        at([-5.0, -12.0, side]),
                        2.5 * k,
                        1.5 * k,
                        26,
                        3,
                        0.3 * k,
                    );
                }
            }
        }
        match rig.hat {
            Hat::None => {}
            Hat::TopHat => {
                self.ellipsoid(at([0.0, 3.0, 0.0]), head.r, mul([5.8, 0.9, 5.0], k), 27, 3);
                // Stacked flat ellipsoids give the crown straight sides and a flat top.
                for i in 0..101 {
                    let y = 4.0 + i as f64 * 0.12;
                    let radius = 3.6 + i as f64 * 0.006;
                    self.ellipsoid(
                        at([0.0, y, 0.0]),
                        head.r,
                        mul([radius, 1.8, radius], k),
                        27,
                        3,
                    );
                }
                self.ellipsoid(at([0.0, 4.6, 0.0]), head.r, mul([3.9, 0.8, 3.9], k), 27, 4);
            }
            Hat::Feathers => {
                self.ellipsoid(at([0.0, 3.5, 0.0]), head.r, mul([4.3, 1.0, 4.0], k), 27, 4);
                for i in -2..=2 {
                    let fan = i as f64;
                    let middle = at([-3.0 - fan.abs(), 10.0, fan * 2.8]);
                    self.sweep(at([0.0, 3.5, fan]), middle, 0.7 * k, 1.5 * k, 27, 4, 0.0);
                    self.sweep(
                        middle,
                        at([-6.0 - fan.abs(), 17.0 - fan.abs() * 2.0, fan * 4.2]),
                        1.5 * k,
                        0.15 * k,
                        27,
                        4,
                        0.0,
                    );
                }
            }
            Hat::Plume => {
                self.ellipsoid(at([-0.5, 3.6, 0.0]), head.r, mul([4.3, 2.0, 3.6], k), 27, 3);
                for side in [-1.0, 1.0] {
                    self.sweep(
                        at([4.8, 2.4, 0.0]),
                        at([-4.0, 4.0, side * 4.8]),
                        0.9 * k,
                        1.4 * k,
                        27,
                        3,
                        0.0,
                    );
                    self.sweep(
                        at([-4.0, 4.0, side * 4.8]),
                        at([-4.0, 3.0, -side * 4.8]),
                        1.4 * k,
                        1.4 * k,
                        27,
                        3,
                        0.0,
                    );
                }
                self.sweep(
                    at([-2.0, 4.0, 2.0]),
                    at([-6.0, 10.0, 2.0]),
                    1.0 * k,
                    2.0 * k,
                    27,
                    4,
                    0.0,
                );
                self.sweep(
                    at([-6.0, 10.0, 2.0]),
                    at([-12.0, 13.0, 2.0]),
                    2.0 * k,
                    0.3 * k,
                    27,
                    4,
                    0.5 * k,
                );
            }
        }
        if rig.mask != Mask::None {
            self.ellipsoid(at([3.1, 0.5, 0.0]), head.r, mul([1.6, 2.0, 3.8], k), 28, 4);
            match rig.mask {
                Mask::None => {}
                Mask::Domino => {
                    for side in [-1.0, 1.0] {
                        self.sweep(
                            at([3.4, 0.5, side * 2.8]),
                            at([4.0, 3.2, side * 6.2]),
                            1.6 * k,
                            0.35 * k,
                            28,
                            4,
                            0.0,
                        );
                    }
                }
                Mask::Beak => {
                    self.sweep(
                        at([3.2, -0.8, 0.0]),
                        at([8.0, -2.5, 0.0]),
                        1.8 * k,
                        1.2 * k,
                        28,
                        4,
                        0.0,
                    );
                    self.sweep(
                        at([8.0, -2.5, 0.0]),
                        at([11.5, -5.0, 0.0]),
                        1.2 * k,
                        0.2 * k,
                        28,
                        4,
                        0.0,
                    );
                }
            }
            for side in [-1.0, 1.0] {
                self.ellipsoid(
                    at([3.8, 0.8, side * 3.1]),
                    head.r,
                    mul([0.8, 0.7, 0.95], k),
                    28,
                    3,
                );
            }
        }
    }
    fn body(&mut self, s: &Solved) {
        let b = &s.b;
        self.frame = 0;
        self.ellipsoid(
            add(b[0].t, mv(b[0].r, [0.0, -0.2, 0.0])),
            b[0].r,
            [2.9, 2.6, 4.2],
            1,
            0,
        );
        self.frame = 1;
        self.sweep(b[0].t, b[2].t, 3.0, 3.3, 2, 1, 0.0);
        self.frame = 2;
        self.ellipsoid(
            add(b[2].t, mv(b[2].r, [0.2, 0.9, 0.0])),
            b[2].r,
            [3.4, 4.2, 5.5],
            3,
            0,
        );
        self.frame = 4;
        self.sweep(
            b[4].t,
            add(b[4].t, mv(b[3].r, [0.0, 1.6, 0.0])),
            1.25,
            1.15,
            4,
            1,
            0.0,
        );
        self.frame = 3;
        self.ellipsoid(b[3].t, b[3].r, [3.0, 3.3, 2.8], 5, 0);
        self.ellipsoid(
            add(b[3].t, mv(b[3].r, [0.85, -0.45, 0.0])),
            b[3].r,
            [2.4, 2.7, 2.35],
            6,
            0,
        );
        for i in 0..2 {
            let a = s.arms[i];
            let sh = s.shoulders[i];
            let part = 7 + i as u8 * 4;
            self.frame = 5 + i as i8 * 2;
            self.sphere(sh, 2.5, part, 0);
            self.sweep(sh, a.joint, 2.1, 1.6, part, 0, 0.0);
            self.sphere(a.joint, 1.45, part + 1, 1);
            self.frame += 1;
            self.sweep(a.joint, a.end, 1.6, 1.2, part + 2, 0, 0.15);
            self.sphere(a.end, 1.9, part + 3, 0);
        }
        for i in 0..2 {
            let l = s.legs[i];
            let part = 15 + i as u8 * 4;
            self.frame = 9 + i as i8 * 2;
            self.sweep(s.hips[i], l.joint, 3.2, 2.2, part, 0, 0.0);
            self.sphere(l.joint, 1.9, part + 1, 1);
            self.frame += 1;
            self.sweep(l.joint, l.end, 2.15, 1.4, part + 2, 0, 0.4);
            self.frame = 13 + i as i8;
            let foot = b[13 + i];
            self.sweep(
                add(foot.t, mv(foot.r, [-1.5, -0.6, 0.0])),
                add(foot.t, mv(foot.r, [4.6, -1.1, 0.0])),
                1.7,
                1.25,
                part + 3,
                0,
                0.0,
            );
        }
    }
    fn finish(mut self, s: &Solved, rest: &Solved, rig: &Rig) -> RenderedPose {
        let mut tones = vec![0usize; BW * BH];
        for &i in &self.touched {
            let v = self.tone[i];
            let part = self.part[i];
            let t = if v > 0.8 {
                0
            } else if v > 0.42 {
                1
            } else if v > 0.08 {
                2
            } else {
                3
            };
            let dim = usize::from((11..=14).contains(&part) || (19..=22).contains(&part));
            tones[i] = (t + dim).min(3);
        }
        for &i in &self.touched {
            if self.mat[i] == 2 {
                continue;
            }
            let t = tones[i];
            let mut votes = [0; 4];
            let mut same = false;
            for j in [i - 1, i + 1, i - BW, i + BW] {
                if self.part[j] != self.part[i] {
                    continue;
                }
                let u = tones[j];
                votes[u] += 1;
                if u == t {
                    same = true;
                }
            }
            if !same && votes.iter().any(|&v| v > 0) {
                let max = *votes.iter().max().unwrap();
                tones[i] = votes.iter().position(|&v| v == max).unwrap();
            }
        }
        for &i in &self.touched {
            if self.mat[i] != 2 {
                self.color[i] = match self.mat[i] {
                    1 => rig.core[tones[i]],
                    3 => rig.costume[tones[i]],
                    4 => rig.trim[tones[i]],
                    _ => rig.shell[tones[i]],
                };
            }
        }
        let mut marks = vec![None; BW * BH];
        for &i in &self.touched {
            for j in [i + 1, i + BW] {
                if self.part[j] == 0 || self.part[j] == self.part[i] {
                    continue;
                }
                let far = if self.z[i] < self.z[j] { i } else { j };
                let dz = (self.z[i] as f64 - self.z[j] as f64).abs();
                let steel = self.mat[i] == 2 || self.mat[j] == 2;
                if self.mat[far] == 2 && dz < 1.0 {
                    continue;
                }
                if steel || dz > 2.2 {
                    marks[far] = Some(INK);
                } else if marks[far].is_none() {
                    marks[far] = Some(SEAM);
                }
            }
        }
        for &i in &self.touched {
            if let Some(c) = marks[i] {
                self.color[i] = c;
            }
        }
        let b = s.b[2];
        let chest = add(b.t, mv(b.r, [3.0, 1.6, -1.8]));
        if mv(b.r, [1.0, 0.0, 0.0])[2] > 0.15
            && let Some(i) = Self::idx(round(chest[0]) as i32, round(chest[1]) as i32)
            && self.part[i] == 3
        {
            self.color[i] = hex(0xf53cd1);
        }
        let mut seen = vec![false; BW * BH];
        let mut ring = vec![];
        for &i in &self.touched {
            for j in [i - 1, i + 1, i - BW, i + BW] {
                if self.part[j] == 0 && !seen[j] {
                    seen[j] = true;
                    ring.push((j, i));
                }
            }
        }
        let source = |i: usize| -> i16 {
            let f = self.fr[i];
            if f < 0 {
                return -1;
            }
            let a = s.b[f as usize];
            let r0 = rest.b[f as usize];
            let p = [
                (i % BW) as f64 - OX as f64,
                OY as f64 - (i / BW) as f64,
                self.z[i] as f64,
            ];
            let lp = tv(a.r, sub(p, a.t));
            let rp = add(r0.t, mv(r0.r, lp));
            let c = round(rp[0]) as i32 + 12;
            let r = 38 - round(rp[1]) as i32;
            if !(0..24).contains(&c) || !(0..38).contains(&r) {
                -1
            } else {
                (r * 24 + c) as i16
            }
        };
        let mut pixels = vec![];
        for &i in &self.touched {
            pixels.push(RigPixel {
                x: (i % BW) as i16 - OX as i16,
                y: (i / BW) as i16 - OY as i16,
                color: self.color[i],
                source: source(i),
                ink: self.color[i] == INK,
            });
        }
        for (j, i) in ring {
            pixels.push(RigPixel {
                x: (j % BW) as i16 - OX as i16,
                y: (j / BW) as i16 - OY as i16,
                color: INK,
                source: source(i),
                ink: true,
            });
        }
        RenderedPose { pixels }
    }
}
const HI: Color = hex(0xffffff);
const A: Color = hex(0xe8edf6);
const B: Color = hex(0xa4afc6);
const D: Color = hex(0x68748f);
const GOLD: Color = hex(0xffd84a);
const GOLD_D: Color = hex(0xb8862a);
const GRIP: Color = hex(0x7a3a2a);
const GRIP_D: Color = hex(0x4a2018);
const WOOD: Color = hex(0x8a5a32);
const WOOD_D: Color = hex(0x5a3820);
struct Handle<'a> {
    r: &'a mut Raster,
    h: V,
    axis: V,
    side: V,
    part: u8,
    clip: bool,
}
impl Handle<'_> {
    fn at(&self, i: f64, k: f64) -> V {
        add(self.h, add(mul(self.axis, i), mul(self.side, k)))
    }
    fn put(&mut self, i: f64, k: f64, c: Color, lift: f64) {
        let n = cross(self.axis, self.side);
        let x = self.h[0] + self.axis[0] * i + self.side[0] * k + n[0] * lift;
        let y = self.h[1] + self.axis[1] * i + self.side[1] * k + n[1] * lift;
        let z = self.h[2] + self.axis[2] * i + self.side[2] * k + n[2] * lift;
        let (wx, wy) = (round(x) as i32, round(y) as i32);
        if self.clip && wy < 0 {
            return;
        }
        self.r.frame = -1;
        self.r.plot(wx, wy, z + 0.05, 2.0, self.part, 2, c);
    }
    fn haft(&mut self, from: f64, to: f64, a: Color, d: Color, r: f64) {
        let mut i = from;
        while i <= to {
            let mut k = -r;
            while k <= r + 0.01 {
                self.put(i, k, if k > 0.1 { d } else { a }, 0.0);
                k += 0.4;
            }
            i += 0.4;
        }
    }
    fn blade(&mut self, from: f64, length: f64, w: f64) {
        let mut i = from;
        while i <= from + length + 0.01 {
            let r = i - from;
            let wd = if r > length - 5.0 {
                (((length - r) / 5.0) * w).max(0.0)
            } else {
                w
            };
            let mut k = -wd;
            while k <= wd + 0.01 {
                self.put(
                    i,
                    k,
                    if k < -w * 0.4 {
                        HI
                    } else if k > w * 0.4 {
                        D
                    } else {
                        A
                    },
                    0.0,
                );
                k += 0.4;
            }
            i += 0.4;
        }
    }
    fn crossbar(&mut self, i: f64, w: f64, c: Color) {
        let mut k = -w;
        while k <= w {
            self.put(i, k, c, 0.0);
            k += 0.4;
        }
    }
    fn line(&mut self, a: V, b: V, c: Color) {
        let n = (len(sub(b, a)) / 0.4).ceil().max(2.0) as usize;
        for k in 0..=n {
            let p = add(a, mul(sub(b, a), k as f64 / n as f64));
            self.r.frame = -1;
            self.r.plot(
                round(p[0]) as i32,
                round(p[1]) as i32,
                p[2] + 0.05,
                2.0,
                self.part,
                2,
                c,
            );
        }
    }
    fn draw(&mut self, w: Weapon, q: &Pose, off: V) {
        match w {
            Weapon::Sword | Weapon::Greatsword => {
                let great = w == Weapon::Greatsword;
                let end = if great {
                    -7.2
                } else if q.two {
                    -6.5
                } else {
                    -2.6
                };
                self.haft(end, 2.2, GRIP, GRIP_D, 0.8);
                self.crossbar(end - 0.4, if great { 1.8 } else { 1.4 }, GOLD);
                let width = if great { 6.6 } else { 4.6 };
                let mut k = -width;
                while k <= width {
                    self.put(
                        2.6,
                        k,
                        if k.abs() > width - 0.8 { GOLD_D } else { GOLD },
                        0.0,
                    );
                    self.put(3.0, k, GOLD_D, 0.0);
                    k += 0.4;
                }
                self.blade(
                    3.2,
                    if great { 31.0 } else { 25.0 },
                    if great { 3.0 } else { 2.2 },
                );
                if great {
                    let mut i = 4.0;
                    while i <= 28.0 {
                        self.put(i, 0.0, B, 0.0);
                        i += 0.4;
                    }
                }
            }
            Weapon::Dagger => {
                self.haft(-2.4, 1.8, GRIP, GRIP_D, 0.8);
                self.crossbar(2.2, 2.6, GOLD_D);
                self.blade(2.6, 10.0, 1.3);
            }
            Weapon::Candelabrum => {
                // A gilt pole, a cross-arm at the head, and three candles burning on it.
                const WAX: Color = hex(0xf0e6cc);
                const WAX_D: Color = hex(0xb8ac90);
                const FLAME: Color = hex(0xffd36a);
                const FLAME_HOT: Color = hex(0xfff6d0);
                const FLAME_EDGE: Color = hex(0xff8a2a);
                self.haft(-14.0, 30.0, GOLD_D, hex(0x7a5418), 0.5);
                // The cross-arm, and a drip-cup at its middle.
                self.crossbar(30.0, 5.6, GOLD);
                self.crossbar(30.4, 5.6, GOLD_D);
                self.crossbar(29.2, 2.0, GOLD);
                for k in [-5.0_f64, 0.0, 5.0] {
                    // A short candle, then a tall flame on its tip.
                    let mut i = 30.8;
                    while i <= 33.2 {
                        self.put(i, k, WAX, 0.0);
                        self.put(i, k + 0.4, WAX_D, 0.0);
                        i += 0.4;
                    }
                    let mut i: f64 = 33.6;
                    while i <= 40.0 {
                        let u = (i - 33.6) / 6.4;
                        // Widest a little above the wick, then tapering to a point.
                        let w: f64 = 1.6 * (1.0 - (u - 0.3).abs() / 0.7).max(0.15);
                        let mut d = -w;
                        while d <= w + 0.01 {
                            let c = if d.abs() > w - 0.5 {
                                FLAME_EDGE
                            } else if u < 0.45 {
                                FLAME_HOT
                            } else {
                                FLAME
                            };
                            self.put(i, k + d, c, 0.0);
                            d += 0.4;
                        }
                        i += 0.4;
                    }
                }
            }
            Weapon::Spear => {
                self.haft(-18.0, 36.0, WOOD, WOOD_D, 0.5);
                self.blade(36.0, 11.0, 2.0);
                self.crossbar(36.0, 2.4, GOLD_D);
            }
            Weapon::Hammer => {
                for lift in [-0.8, 0.0, 0.8] {
                    let mut i = -10.0;
                    while i <= 18.0 {
                        let mut k = -0.8;
                        while k <= 0.81 {
                            self.put(i, k, if k > 0.1 { WOOD_D } else { WOOD }, lift);
                            k += 0.4;
                        }
                        i += 0.4;
                    }
                }
                let mut i = 18.0;
                while i <= 28.0 {
                    let mut k: f64 = -9.0;
                    while k <= 9.0 {
                        let face = if k.abs() > 7.4 {
                            if k < 0.0 { A } else { D }
                        } else if i < 20.0 {
                            HI
                        } else if i > 26.4 {
                            D
                        } else if k.abs() < 2.0 {
                            GOLD
                        } else {
                            B
                        };
                        self.put(i, k, face, 4.5);
                        self.put(i, k, face, -4.5);
                        if i < 18.4 || i > 27.6 || k.abs() > 8.6 {
                            let mut l = -4.0;
                            while l <= 4.0 {
                                self.put(
                                    i,
                                    k,
                                    if k < 0.0 {
                                        A
                                    } else if i > 27.6 {
                                        D
                                    } else {
                                        B
                                    },
                                    l,
                                );
                                l += 0.4;
                            }
                        }
                        k += 0.4;
                    }
                    i += 0.4;
                }
            }
            Weapon::Bottle => {
                // Glass from the foiled neck in the hand out to the base: a narrow neck,
                // a sloping shoulder, then the full body with a label band, round in depth.
                const GLASS: Color = hex(0x2c5a3c);
                const GLASS_D: Color = hex(0x173222);
                const GLASS_HI: Color = hex(0x8cc49a);
                const LABEL: Color = hex(0xe8dcc0);
                const LABEL_D: Color = hex(0xb8a888);
                let mut i = -7.0;
                while i <= 30.0 {
                    let w: f64 = if i < 4.0 {
                        1.4
                    } else if i < 10.0 {
                        1.4 + (i - 4.0) / 6.0 * 3.4
                    } else {
                        4.8
                    };
                    let mut k = -w;
                    while k <= w + 0.01 {
                        let edge = k.abs() > w - 0.6;
                        let c = if i < 4.0 {
                            if k > 0.3 { GOLD_D } else { GOLD }
                        } else if (17.0..23.0).contains(&i) && !edge {
                            if k > 1.5 { LABEL_D } else { LABEL }
                        } else if i > 29.2 {
                            GLASS_D
                        } else if (-2.6..-1.4).contains(&k) {
                            GLASS_HI
                        } else if k > w * 0.35 || edge {
                            GLASS_D
                        } else {
                            GLASS
                        };
                        // Round in depth: thicker through the middle of the bottle.
                        let depth = (w * w - k * k).max(0.0).sqrt();
                        let mut l = -depth;
                        while l <= depth + 0.01 {
                            self.put(i, k, c, l);
                            l += 0.8;
                        }
                        k += 0.4;
                    }
                    i += 0.4;
                }
            }
            Weapon::Scythe => {
                self.haft(-16.0, 31.0, hex(0x3a3448), hex(0x262033), 0.5);
                let mut j = 0.0;
                while j <= 28.0 {
                    let u = j / 28.0;
                    let w = 3.2 * (1.0 - u) + 0.3;
                    let b = 31.5 - u * u * 12.0;
                    let mut k = -w;
                    while k <= w {
                        self.put(
                            b + k,
                            j + 0.8,
                            if k > w * 0.3 {
                                HI
                            } else if k < -w * 0.4 {
                                D
                            } else {
                                B
                            },
                            0.0,
                        );
                        k += 0.4;
                    }
                    j += 0.4;
                }
            }
            Weapon::Claw => {
                for q0 in [-2.4, 0.0, 2.4] {
                    let mut i = 2.0;
                    while i <= 9.5 {
                        let u = (i - 2.0) / 7.5;
                        let k = q0 + u * u * 1.8;
                        self.put(i, k, if u > 0.55 { HI } else { A }, 0.0);
                        if u < 0.6 {
                            self.put(i, k + 0.8, D, 0.0);
                        }
                        i += 0.4;
                    }
                }
            }
            Weapon::Bow => {
                let mut u: f64 = -1.0;
                while u <= 1.001 {
                    let k = u * 17.0;
                    let i = 6.0 * (1.0 - u * u) - 1.0;
                    self.put(i, k, if u.abs() > 0.85 { GOLD_D } else { WOOD }, 0.0);
                    self.put(i + 0.8, k, WOOD_D, 0.0);
                    u += 0.025;
                }
                let ta = self.at(-1.0, -17.0);
                let tb = self.at(-1.0, 17.0);
                let mid = if q.pull { off } else { self.at(-1.0, 0.0) };
                self.line(ta, mid, A);
                self.line(tb, mid, A);
                if q.nocked {
                    self.line(mid, self.at(13.0, 0.0), WOOD);
                    let mut i = 9.0;
                    while i <= 13.0 {
                        let mut k = -(13.0 - i) * 0.45;
                        while k <= (13.0 - i) * 0.45 {
                            self.put(i, k, HI, 0.0);
                            k += 0.4;
                        }
                        i += 0.4;
                    }
                }
            }
            Weapon::Pistol => {
                let mut i = -1.0;
                while i <= 9.5 {
                    let mut k = -4.2;
                    while k <= -2.4 {
                        self.put(
                            i,
                            k,
                            if k < -3.6 {
                                A
                            } else if k > -2.9 {
                                D
                            } else {
                                B
                            },
                            0.0,
                        );
                        k += 0.4;
                    }
                    i += 0.4;
                }
                let mut i = -1.2;
                while i <= 1.2 {
                    let mut k = -2.4;
                    while k <= 2.4 {
                        self.put(i, k, GRIP_D, 0.0);
                        k += 0.4;
                    }
                    i += 0.4;
                }
                self.put(9.0, -4.6, HI, 0.0);
            }
            Weapon::Launcher => {
                let mut i: f64 = -14.0;
                while i <= 22.0 {
                    let mut k = -6.0;
                    while k <= -1.0 {
                        self.put(
                            i,
                            k,
                            if k < -4.8 {
                                hex(0x7d8c58)
                            } else if k > -2.2 {
                                hex(0x3a4529)
                            } else if (i - 3.0).abs() < 1.4 {
                                GOLD
                            } else {
                                hex(0x58663f)
                            },
                            0.0,
                        );
                        k += 0.4;
                    }
                    i += 0.4;
                }
                let mut k = -6.8;
                while k <= -0.2 {
                    self.put(22.4, k, B, 0.0);
                    self.put(-14.4, k, D, 0.0);
                    k += 0.4;
                }
            }
            Weapon::Coupe => {
                self.haft(0.0, 5.0, B, D, 0.5);
                for i in -4i32..=4 {
                    self.put(6.0, i as f64, B, 0.0);
                    self.put(5.0 - (i.abs() as f64 * 0.5), i as f64, GOLD, 0.0);
                }
                for i in -2..=2 {
                    self.put(0.0, i as f64, B, 0.0);
                }
            }
            Weapon::Handle => {
                self.haft(-2.6, 6.0, GRIP, GRIP_D, 0.8);
                let mut k = -1.6;
                while k <= 1.6 {
                    self.put(-3.0, k, GOLD, 0.0);
                    self.put(6.4, k, GOLD, 0.0);
                    k += 0.4;
                }
            }
        }
    }
}
impl Rig {
    /// Solves and renders a pose. The result is local to the feet; placement belongs to the painter.
    pub fn render(&self, q: &Pose) -> RenderedPose {
        let mut s = solve(q, &self.measurements);
        let whole = q.yaw + if q.face == -1 { PI } else { 0.0 };
        if whole != 0.0 {
            s.turn(whole);
        }
        let clip = q.y >= -0.5;
        let mut r = Raster::new(self.light);
        r.clip = clip;
        r.body(&s);
        if self.belly > 0.0 {
            r.frame = 1;
            let waist = s.b[1];
            r.ellipsoid(
                add(waist.t, mv(waist.r, [self.belly * 0.45, 0.0, 0.0])),
                waist.r,
                [self.belly, self.belly * 1.05, self.belly * 1.1],
                2,
                0,
            );
        }
        r.costume(&s, self);
        if self.skirt > 0.0 {
            r.frame = 0;
            let hip = s.b[0];
            for i in 0..=(self.skirt * 2.0) as usize {
                let u = i as f64 / (self.skirt * 2.0);
                r.ellipsoid(
                    add(hip.t, [0.0, -self.skirt * u, 0.0]),
                    hip.r,
                    [3.5 + 9.5 * u, 0.7, 3.0 + 5.0 * u],
                    30,
                    3,
                );
            }
        }
        if self.ribbons > 0.0 {
            r.frame = 0;
            let hip = s.b[0];
            for side in [-1.0, 1.0] {
                let at = |v| add(hip.t, mv(hip.r, v));
                let middle = at([-self.ribbons * 0.55, -3.0, side * 7.0]);
                r.sweep(at([-2.0, 3.0, side * 3.0]), middle, 1.8, 1.4, 31, 4, 0.0);
                r.sweep(
                    middle,
                    at([-self.ribbons, -12.0, side * 11.0]),
                    1.4,
                    0.5,
                    31,
                    3,
                    0.0,
                );
            }
        }
        if self.tails > 0.0 {
            r.tails(&s, self.tails);
        }
        r.clip = false;
        if let Some(w) = q.weapon {
            Handle {
                r: &mut r,
                h: s.arms[0].end,
                axis: s.axis,
                side: s.side,
                part: 23,
                clip,
            }
            .draw(w, q, s.arms[1].end);
        }
        if let Some(w) = q.off_weapon {
            let mut axis = [q.sb.cos(), -q.sb.sin(), 0.0];
            let mut side = [-q.sb.sin(), -q.sb.cos(), 0.0];
            if whole != 0.0 {
                axis = mv(ry(-whole), axis);
                side = mv(ry(-whole), side);
            }
            Handle {
                r: &mut r,
                h: s.arms[1].end,
                axis,
                side,
                part: 24,
                clip,
            }
            .draw(w, q, s.arms[1].end);
        }
        let rest = solve(&Pose::default(), &self.measurements);
        r.finish(&s, &rest, self)
    }
    /// Returns solved joint positions in screen coordinates, with z toward the viewer.
    pub fn skeleton(&self, q: &Pose) -> Vec<(&'static str, [f64; 3])> {
        let mut s = solve(q, &self.measurements);
        s.turn(q.yaw + if q.face == -1 { PI } else { 0.0 });
        let mut out = vec![
            ("pelvis", s.b[0].t),
            ("waist", s.b[1].t),
            ("chest", s.b[2].t),
            ("neck", s.b[4].t),
            ("head", s.b[3].t),
        ];
        for (i, names) in [
            ["shoulderN", "elbowN", "handN", "hipN", "kneeN", "ankleN"],
            ["shoulderF", "elbowF", "handF", "hipF", "kneeF", "ankleF"],
        ]
        .iter()
        .enumerate()
        {
            out.extend(names.iter().copied().zip([
                s.shoulders[i],
                s.arms[i].joint,
                s.arms[i].end,
                s.hips[i],
                s.legs[i].joint,
                s.legs[i].end,
            ]));
        }
        for (_, v) in &mut out {
            v[1] = -v[1];
        }
        out
    }
    pub fn shortfall(&self, q: &Pose) -> f64 {
        let s = solve(q, &self.measurements);
        let near = len(sub(s.arms[0].end, [q.fx, -q.fy, s.arms[0].end[2]]));
        let far = if q.two {
            0.0
        } else {
            len(sub(s.arms[1].end, [q.bx, -q.by, s.arms[1].end[2]]))
        };
        near.max(far)
    }
}
