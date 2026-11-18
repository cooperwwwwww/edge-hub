//! Input engine: parses DS4 reports and turns them into a virtual Xbox 360 report.
//! Every step is a pure function of the current sample, so nothing adds delay.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub const SOURCES: [&str; 18] = [
    "cross", "circle", "square", "triangle", "l1", "r1", "l2", "r2", "l3", "r3", "share", "options", "ps", "touch",
    "up", "down", "left", "right",
];
pub const TARGETS: [&str; 19] = [
    "A", "B", "X", "Y", "LB", "RB", "LT", "RT", "LS", "RS", "BACK", "START", "GUIDE", "TOUCHPAD", "UP", "DOWN", "LEFT", "RIGHT", "NONE",
];
pub const DEFAULT_MAP: [&str; 18] = [
    "A", "B", "X", "Y", "LB", "RB", "LT", "RT", "LS", "RS", "BACK", "START", "GUIDE", "TOUCHPAD", "UP", "DOWN", "LEFT", "RIGHT",
];
const L2: usize = 6;
const R2: usize = 7;
const PS: usize = 12;
const L1: usize = 4;
const UP: usize = 14;

/// Windows virtual-key code for a key name used in "KEY:<name>" targets.
pub fn vk(name: &str) -> u16 {
    let n = name.to_uppercase();
    if n.len() == 1 {
        let c = n.as_bytes()[0];
        if c.is_ascii_uppercase() || c.is_ascii_digit() { return c as u16; }
    }
    if let Some(num) = n.strip_prefix('F').and_then(|x| x.parse::<u16>().ok()) {
        if (1..=24).contains(&num) { return 0x6F + num; }
    }
    match n.as_str() {
        "SPACE" => 0x20, "ENTER" => 0x0D, "ESC" => 0x1B, "TAB" => 0x09, "BACKSPACE" => 0x08, "CAPSLOCK" => 0x14,
        "SHIFT" => 0xA0, "RSHIFT" => 0xA1, "CTRL" => 0xA2, "RCTRL" => 0xA3, "ALT" => 0xA4, "RALT" => 0xA5, "WIN" => 0x5B,
        "UP" => 0x26, "DOWN" => 0x28, "LEFT" => 0x25, "RIGHT" => 0x27, "INSERT" => 0x2D, "DELETE" => 0x2E,
        "HOME" => 0x24, "END" => 0x23, "PAGEUP" => 0x21, "PAGEDOWN" => 0x22, "PRINTSCREEN" => 0x2C,
        "VOLUP" => 0xAF, "VOLDOWN" => 0xAE, "MUTE" => 0xAD, "PLAYPAUSE" => 0xB3, "NEXT" => 0xB0, "PREV" => 0xB1,
        "MINUS" => 0xBD, "EQUALS" => 0xBB, "COMMA" => 0xBC, "PERIOD" => 0xBE, "SLASH" => 0xBF, "SEMICOLON" => 0xBA,
        "QUOTE" => 0xDE, "LBRACKET" => 0xDB, "RBRACKET" => 0xDD, "BACKSLASH" => 0xDC, "GRAVE" => 0xC0,
        _ => 0,
    }
}

pub fn valid_target(t: &str) -> bool {
    TARGETS.contains(&t)
        || t.strip_prefix("KEY:").map_or(false, |k| vk(k) != 0)
        || t.strip_prefix("KEYS:").map_or(false, |k| !k.is_empty() && k.split('+').all(|x| vk(x) != 0))
        || matches!(t, "MOUSE_L" | "MOUSE_R" | "MOUSE_M" | "MOUSE_X1" | "MOUSE_X2" | "WHEEL_UP" | "WHEEL_DOWN")
}

pub fn xusb(t: &str) -> u16 {
    match t {
        "UP" => 0x0001, "DOWN" => 0x0002, "LEFT" => 0x0004, "RIGHT" => 0x0008, "START" => 0x0010, "BACK" => 0x0020,
        "LS" => 0x0040, "RS" => 0x0080, "LB" => 0x0100, "RB" => 0x0200, "GUIDE" => 0x0400, "TOUCHPAD" => 0x0800,
        "A" => 0x1000, "B" => 0x2000, "X" => 0x4000, "Y" => 0x8000, _ => 0,
    }
}

// ------------------------------------------------------------------ profile JSON

fn stick_default() -> Value {
    json!({"shape":"radial","inner":0.06,"inner_x":0.06,"inner_y":0.06,"outer":0.98,"curve":"linear",
           "sens":1.0,"sens_x":1.0,"sens_y":1.0,"anti":0.0,"anti_x":0.0,"anti_y":0.0,
           "points":[[0.25,0.25],[0.5,0.5],[0.75,0.75]],"y0":0.0,"y1":1.0,"smooth":false,
           "invx":false,"invy":false,"rotation":0,"snap":0,"square":false,"resolution":0,"jitter":0.0,
           "hires":0.0,"snapback":false,"ads_sens":1.0,"shake":0.0,"shake_freq":30})
}
fn trig_default() -> Value {
    json!({"start":0.02,"end":1.0,"anti":0.0,"digital":false,"curve":"linear",
           "points":[[0.25,0.25],[0.5,0.5],[0.75,0.75]],"y0":0.0,"y1":1.0,"smooth":false})
}
pub fn default_map() -> Value {
    let mut m = serde_json::Map::new();
    for (i, s) in SOURCES.iter().enumerate() {
        m.insert(s.to_string(), json!(DEFAULT_MAP[i]));
    }
    Value::Object(m)
}
pub fn default_profile(name: &str, color: &str) -> Value {
    json!({
        "name": name, "color": color, "map": default_map(), "turbo": {}, "toggle": {}, "turbo_rate": 12, "games": [],
        "mouse": {"stick": "none", "speed": 1400, "accel": 1.6, "touchpad": false, "touch_sens": 1.0},
        "sticks": {"left": stick_default(), "right": stick_default()}, "swap_sticks": false,
        "shift": {"button": "none", "layer": {}},
        "triggers": {"left": trig_default(), "right": trig_default()},
        "rumble": {"intensity": 1.0, "swap": false},
        "gyro": {"enabled": false, "mode": "hold_l2", "sens": 1.0, "invx": false, "invy": false},
        "macros": [], "light": {"battery": false, "brightness": 1.0, "mode": "static", "speed": 1.0}
    })
}

pub fn deep_merge(base: &Value, over: &Value) -> Value {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            let mut out = b.clone();
            for (k, v) in o {
                let merged = match (b.get(k), v) {
                    (Some(bv @ Value::Object(_)), Value::Object(_)) if k != "map" && k != "turbo" && k != "toggle" => deep_merge(bv, v),
                    _ => v.clone(),
                };
                out.insert(k.clone(), merged);
            }
            if let Some(Value::Object(m)) = o.get("map") {
                let mut dm = default_map();
                for (k, v) in m {
                    if SOURCES.contains(&k.as_str()) && v.as_str().map_or(false, valid_target) {
                        dm[k.as_str()] = v.clone();
                    }
                }
                out.insert("map".into(), dm);
            }
            Value::Object(out)
        }
        _ => over.clone(),
    }
}

fn f(v: &Value, k: &str, d: f64) -> f64 { v.get(k).and_then(|x| x.as_f64()).unwrap_or(d) }
fn b(v: &Value, k: &str) -> bool { v.get(k).and_then(|x| x.as_bool()).unwrap_or(false) }
fn s<'a>(v: &'a Value, k: &str, d: &'a str) -> &'a str { v.get(k).and_then(|x| x.as_str()).unwrap_or(d) }

// ------------------------------------------------------------------ curves

#[derive(Clone, Debug)]
pub enum Curve {
    Pow(f64),
    Custom { xs: Vec<f64>, ys: Vec<f64>, sl: Option<Vec<f64>> },
}

impl Curve {
    pub fn from(v: &Value) -> Curve {
        let name = s(v, "curve", "linear");
        if name != "custom" {
            return Curve::Pow(match name { "smooth" => 1.4, "precise" => 1.9, "aggressive" => 0.65, "instant" => 0.4, _ => 1.0 });
        }
        let mut pts: Vec<(f64, f64)> = vec![(0.0, f(v, "y0", 0.0))];
        if let Some(Value::Array(a)) = v.get("points") {
            for p in a {
                if let (Some(x), Some(y)) = (p.get(0).and_then(|x| x.as_f64()), p.get(1).and_then(|y| y.as_f64())) {
                    let x = x.clamp(0.0, 1.0);
                    if x > 0.0 && x < 1.0 {
                        pts.push((x, y.clamp(0.0, 1.0)));
                    }
                }
            }
        }
        pts.push((1.0, f(v, "y1", 1.0)));
        pts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
        let mut sl = None;
        if b(v, "smooth") && xs.len() > 2 {
            // Fritsch-Carlson monotone cubic
            let n = xs.len();
            let d: Vec<f64> = (0..n - 1).map(|k| (ys[k + 1] - ys[k]) / (xs[k + 1] - xs[k]).max(1e-9)).collect();
            let mut m = vec![d[0]];
            for k in 1..n - 1 {
                m.push(if d[k - 1] * d[k] <= 0.0 { 0.0 } else { (d[k - 1] + d[k]) / 2.0 });
            }
            m.push(d[n - 2]);
            for k in 0..n - 1 {
                if d[k] == 0.0 {
                    m[k] = 0.0;
                    m[k + 1] = 0.0;
                } else {
                    let (a, bb) = (m[k] / d[k], m[k + 1] / d[k]);
                    let h = a * a + bb * bb;
                    if h > 9.0 {
                        let t = 3.0 / h.sqrt();
                        m[k] = t * a * d[k];
                        m[k + 1] = t * bb * d[k];
                    }
                }
            }
            sl = Some(m);
        }
        Curve::Custom { xs, ys, sl }
    }

    #[inline]
    pub fn apply(&self, n: f64) -> f64 {
        match self {
            Curve::Pow(e) => n.powf(*e),
            Curve::Custom { xs, ys, sl } => {
                for k in 1..xs.len() {
                    if n <= xs[k] {
                        let (x0, x1, y0, y1) = (xs[k - 1], xs[k], ys[k - 1], ys[k]);
                        let h = x1 - x0;
                        if h <= 0.0 {
                            return y1;
                        }
                        let t = (n - x0) / h;
                        return match sl {
                            None => y0 + (y1 - y0) * t,
                            Some(m) => {
                                let (t2, t3) = (t * t, t * t * t);
                                ((2.0 * t3 - 3.0 * t2 + 1.0) * y0 + (t3 - 2.0 * t2 + t) * h * m[k - 1]
                                    + (-2.0 * t3 + 3.0 * t2) * y1 + (t3 - t2) * h * m[k])
                                    .clamp(0.0, 1.0)
                            }
                        };
                    }
                }
                *ys.last().unwrap_or(&1.0)
            }
        }
    }
}

#[inline]
fn anti(n: f64, a: f64) -> f64 { if a > 0.0 && n > 0.0 { a + (1.0 - a) * n } else { n } }
#[inline]
fn rescale(v: f64, inner: f64, outer: f64) -> f64 {
    if v <= inner { 0.0 } else { ((v - inner) / (outer - inner).max(0.01)).min(1.0) }
}

// ------------------------------------------------------------------ typed config

#[derive(Clone, Debug)]
pub struct StickCfg {
    shape: u8, inner: f64, inner_x: f64, inner_y: f64, outer: f64, curve: Curve, sens: f64, sens_x: f64, sens_y: f64,
    anti: f64, anti_x: f64, anti_y: f64, invx: bool, invy: bool, rotation: f64, snap: f64, square: bool,
    resolution: i32, jitter: f64, hires: f64, snapback: bool, ads_sens: f64, shake: f64, shake_freq: f64,
}
impl StickCfg {
    fn from(v: &Value) -> Self {
        StickCfg {
            shape: match s(v, "shape", "radial") { "axial" => 1, "hybrid" => 2, _ => 0 },
            inner: f(v, "inner", 0.06), inner_x: f(v, "inner_x", 0.06), inner_y: f(v, "inner_y", 0.06),
            outer: f(v, "outer", 0.98), curve: Curve::from(v), sens: f(v, "sens", 1.0), sens_x: f(v, "sens_x", 1.0),
            sens_y: f(v, "sens_y", 1.0), anti: f(v, "anti", 0.0), anti_x: f(v, "anti_x", 0.0), anti_y: f(v, "anti_y", 0.0),
            invx: b(v, "invx"), invy: b(v, "invy"), rotation: f(v, "rotation", 0.0), snap: f(v, "snap", 0.0),
            square: b(v, "square"), resolution: f(v, "resolution", 0.0) as i32, jitter: f(v, "jitter", 0.0),
            hires: f(v, "hires", 0.0).clamp(0.0, 1.0), snapback: b(v, "snapback"), ads_sens: f(v, "ads_sens", 1.0),
            shake: f(v, "shake", 0.0).clamp(0.0, 0.2), shake_freq: f(v, "shake_freq", 30.0).clamp(1.0, 120.0),
        }
    }
}

pub fn proc_stick(mut x: f64, mut y: f64, c: &StickCfg, center: [f64; 2], last: Option<(f64, f64)>) -> (f64, f64) {
    x -= center[0];
    y -= center[1];
    if c.rotation != 0.0 {
        let a = c.rotation.to_radians();
        let (ca, sa) = (a.cos(), a.sin());
        (x, y) = (x * ca - y * sa, x * sa + y * ca);
    }
    if c.invx { x = -x; }
    if c.invy { y = -y; }
    let outer = c.outer;
    let (mut ox, mut oy);
    if c.shape == 1 {
        let ax = |v: f64, dz: f64| {
            let n = rescale(v.abs(), dz, outer);
            if n == 0.0 { 0.0 } else { anti(c.curve.apply(n), c.anti).copysign(v) }
        };
        ox = ax(x, c.inner_x);
        oy = ax(y, c.inner_y);
    } else {
        let m = x.hypot(y);
        let n = rescale(m, c.inner, outer);
        if n == 0.0 || m == 0.0 {
            ox = 0.0;
            oy = 0.0;
        } else {
            let n = anti(c.curve.apply(n), c.anti);
            ox = x / m * n;
            oy = y / m * n;
            if c.shape == 2 {
                if ox.abs() < c.inner_x * 0.5 { ox = 0.0; }
                if oy.abs() < c.inner_y * 0.5 { oy = 0.0; }
            }
        }
    }
    if c.anti_x > 0.0 && ox != 0.0 { ox = anti(ox.abs(), c.anti_x).copysign(ox); }
    if c.anti_y > 0.0 && oy != 0.0 { oy = anti(oy.abs(), c.anti_y).copysign(oy); }
    ox *= c.sens * c.sens_x;
    oy *= c.sens * c.sens_y;
    if c.snap > 0.0 && (ox != 0.0 || oy != 0.0) {
        let m = ox.hypot(oy);
        let ang = oy.atan2(ox).to_degrees();
        let near = (ang / 90.0).round() * 90.0;
        if (ang - near).abs() <= c.snap {
            let r = near.to_radians();
            ox = m * r.cos();
            oy = m * r.sin();
        }
    }
    if c.square && (ox != 0.0 || oy != 0.0) {
        let k = ox.hypot(oy) / ox.abs().max(oy.abs());
        ox *= k;
        oy *= k;
    }
    ox = ox.clamp(-1.0, 1.0);
    oy = oy.clamp(-1.0, 1.0);
    if c.resolution > 0 {
        let r = c.resolution as f64;
        ox = (ox * r).round() / r;
        oy = (oy * r).round() / r;
    }
    if c.jitter > 0.0 && (ox != 0.0 || oy != 0.0) {
        if let Some(l) = last {
            if (ox - l.0).abs() < c.jitter && (oy - l.1).abs() < c.jitter {
                return l;
            }
        }
    }
    (ox, oy)
}

/// Negative anti-jitter: small, quick stick changes are amplified so micro-adjustments register harder.
/// `j` is the anti-jitter setting (only < 0 does anything here). Adds no delay: it only looks at the
/// difference from the previous un-boosted sample, so the stick settles exactly where it really is.
pub fn micro_boost(cur: (f64, f64), prev: Option<(f64, f64)>, j: f64) -> (f64, f64) {
    if j >= 0.0 { return cur; }
    let Some(p) = prev else { return cur };
    let k = (-j) * 40.0; // -0.05 -> up to 3x on tiny moves
    let ax = |c: f64, p: f64| {
        let d = c - p;
        if c == 0.0 || d.abs() >= 0.08 { c } else { (c + d * k).clamp(-1.0, 1.0) }
    };
    (ax(cur.0, p.0), ax(cur.1, p.1))
}

#[derive(Clone, Debug)]
pub struct TrigCfg { start: f64, end: f64, anti: f64, digital: bool, curve: Curve }
impl TrigCfg {
    fn from(v: &Value) -> Self {
        TrigCfg { start: f(v, "start", 0.0), end: f(v, "end", 1.0), anti: f(v, "anti", 0.0), digital: b(v, "digital"), curve: Curve::from(v) }
    }
}
pub fn proc_trigger(raw: u8, c: &TrigCfg) -> f64 {
    let v = raw as f64 / 255.0;
    let end = c.end.max(c.start + 0.01);
    if v <= c.start { return 0.0; }
    if c.digital { return 1.0; }
    anti(c.curve.apply(((v - c.start) / (end - c.start)).min(1.0)), c.anti)
}

#[derive(Clone, Debug)]
enum Tgt { Btn(u16), Lt, Rt, Key(u16), Keys(Vec<u16>), Mouse(u8), Wheel(i32), None }

#[derive(Clone, Debug)]
struct Step { mask: u16, lt: bool, rt: bool, keys: Vec<u16>, mouse: u8, ms: f64 }
#[derive(Clone, Debug)]
struct Macro { trigger: usize, steps: Vec<Step>, total: f64, repeat: bool, block: bool }

fn parse_macro(text: &str) -> Vec<Step> {
    let mut out = vec![];
    for part in text.replace('\n', ";").split(';') {
        let part = part.trim();
        if part.is_empty() { continue; }
        let mut bits: Vec<&str> = part.split_whitespace().collect();
        let mut ms = 50.0;
        if bits.len() >= 2 {
            if let Ok(v) = bits[bits.len() - 1].parse::<i64>() {
                ms = v.clamp(1, 10000) as f64;
                bits.pop();
            }
        }
        let name = bits.join(" ").to_uppercase();
        if ["WAIT", "PAUSE", "DELAY", ""].contains(&name.as_str()) {
            out.push(Step { mask: 0, lt: false, rt: false, keys: vec![], mouse: 0, ms });
            continue;
        }
        let mut st = Step { mask: 0, lt: false, rt: false, keys: vec![], mouse: 0, ms };
        let mut any = false;
        for t in name.split('+').map(|t| t.trim()) {
            match t {
                "LT" => { st.lt = true; any = true; }
                "RT" => { st.rt = true; any = true; }
                "MOUSE_L" => { st.mouse |= 1; any = true; }
                "MOUSE_R" => { st.mouse |= 2; any = true; }
                "MOUSE_M" => { st.mouse |= 4; any = true; }
                _ if t.starts_with("KEY:") && vk(&t[4..]) != 0 => { st.keys.push(vk(&t[4..])); any = true; }
                _ if xusb(t) != 0 => { st.mask |= xusb(t); any = true; }
                _ => {}
            }
        }
        if any { out.push(st); }
    }
    out
}

fn parse_tgt(t: &str) -> Tgt {
    match t {
        "LT" => Tgt::Lt, "RT" => Tgt::Rt, "NONE" => Tgt::None,
        "MOUSE_L" => Tgt::Mouse(1), "MOUSE_R" => Tgt::Mouse(2), "MOUSE_M" => Tgt::Mouse(4),
        "MOUSE_X1" => Tgt::Mouse(8), "MOUSE_X2" => Tgt::Mouse(16),
        "WHEEL_UP" => Tgt::Wheel(1), "WHEEL_DOWN" => Tgt::Wheel(-1),
        x if x.starts_with("KEYS:") => Tgt::Keys(x[5..].split('+').map(vk).filter(|k| *k != 0).collect()),
        x if x.starts_with("KEY:") => Tgt::Key(vk(&x[4..])),
        x => Tgt::Btn(xusb(x)),
    }
}

#[derive(Clone, Debug)]
pub struct Cfg {
    shift_btn: Option<usize>, shift_map: Vec<Option<Tgt>>,
    map: [Tgt; 18], turbo: [bool; 18], toggle: [bool; 18], turbo_rate: f64,
    mouse_stick: u8, mouse_speed: f64, mouse_accel: f64, touch_mouse: bool, touch_sens: f64,
    pub light_mode: u8, pub light_speed: f64, left: StickCfg, right: StickCfg, swap: bool,
    lt: TrigCfg, rt: TrigCfg, pub rumble_int: f64, pub rumble_swap: bool,
    gyro_on: bool, gyro_mode: u8, gyro_sens: f64, gyro_invx: bool, gyro_invy: bool,
    macros: Vec<Macro>, pub color: (u8, u8, u8), pub light_battery: bool, pub brightness: f64,
}

pub fn hex_rgb(h: &str) -> (u8, u8, u8) {
    let h = h.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    if h.len() >= 6 { (p(0), p(2), p(4)) } else { (255, 42, 61) }
}

impl Cfg {
    pub fn from(p: &Value) -> Cfg {
        let mut map = std::array::from_fn(|_| Tgt::None);
        let mut turbo = [false; 18];
        let mut toggle = [false; 18];
        for (i, src) in SOURCES.iter().enumerate() {
            let t = p["map"].get(*src).and_then(|v| v.as_str()).unwrap_or(DEFAULT_MAP[i]);
            map[i] = parse_tgt(t);
            turbo[i] = p["turbo"].get(*src).and_then(|v| v.as_bool()).unwrap_or(false);
            toggle[i] = p["toggle"].get(*src).and_then(|v| v.as_bool()).unwrap_or(false);
        }
        let mut macros = vec![];
        if let Some(Value::Array(ms)) = p.get("macros") {
            for m in ms {
                let steps = parse_macro(s(m, "steps", ""));
                if let Some(ti) = SOURCES.iter().position(|x| *x == s(m, "trigger", "")) {
                    if !steps.is_empty() {
                        let total = steps.iter().map(|s| s.ms).sum();
                        macros.push(Macro { trigger: ti, steps, total, repeat: b(m, "repeat"),
                            block: m.get("block").and_then(|v| v.as_bool()).unwrap_or(true) });
                    }
                }
            }
        }
        let sh = &p["shift"];
        let shift_btn = SOURCES.iter().position(|x| *x == s(sh, "button", "none"));
        let shift_map: Vec<Option<Tgt>> = SOURCES.iter().map(|src| sh["layer"].get(*src).and_then(|v| v.as_str())
            .filter(|t| valid_target(t)).map(parse_tgt)).collect();
        let g = &p["gyro"];
        let mo = &p["mouse"];
        let li = &p["light"];
        Cfg {
            shift_btn, shift_map,
            map, turbo, toggle, turbo_rate: f(p, "turbo_rate", 12.0),
            mouse_stick: match s(mo, "stick", "none") { "left" => 1, "right" => 2, _ => 0 },
            mouse_speed: f(mo, "speed", 1400.0), mouse_accel: f(mo, "accel", 1.6),
            touch_mouse: b(mo, "touchpad"), touch_sens: f(mo, "touch_sens", 1.0),
            light_mode: match s(li, "mode", "static") { "breathe" => 1, "rainbow" => 2, "battery" => 3, _ => 0 },
            light_speed: f(li, "speed", 1.0),
            left: StickCfg::from(&p["sticks"]["left"]), right: StickCfg::from(&p["sticks"]["right"]),
            swap: b(p, "swap_sticks"),
            lt: TrigCfg::from(&p["triggers"]["left"]), rt: TrigCfg::from(&p["triggers"]["right"]),
            rumble_int: f(&p["rumble"], "intensity", 1.0), rumble_swap: b(&p["rumble"], "swap"),
            gyro_on: b(g, "enabled"),
            gyro_mode: match s(g, "mode", "hold_l2") { "always" => 0, "hold_l1" => 2, "touch" => 3, _ => 1 },
            gyro_sens: f(g, "sens", 1.0), gyro_invx: b(g, "invx"), gyro_invy: b(g, "invy"),
            macros, color: hex_rgb(s(p, "color", "#ff2a3d")),
            light_battery: b(&p["light"], "battery"), brightness: f(&p["light"], "brightness", 1.0),
        }
    }
}

// ------------------------------------------------------------------ raw input

#[derive(Clone, Debug, Default)]
pub struct Raw {
    pub lx: u8, pub ly: u8, pub rx: u8, pub ry: u8, pub l2v: u8, pub r2v: u8,
    pub btn: [bool; 18], pub full: bool, pub gyro: [i16; 3], pub accel: [i16; 3],
    pub battery: Option<u8>, pub charging: bool, pub touches: Vec<(u16, u16)>,
    /// Native stick values (-1..1, Y down = positive like the PS4 report) when the controller has more than
    /// 8 bits per axis (Xbox / XInput = 16 bits). None = use the 8-bit values.
    pub fx: Option<[f64; 4]>,
}

/// PS5 DualSense / DualSense Edge input report (USB 0x01 or Bluetooth 0x31).
pub fn parse_dualsense(r: &[u8]) -> Option<Raw> {
    if r.is_empty() { return None; }
    let b = match r[0] {
        0x01 if r.len() >= 54 => r,
        0x31 if r.len() >= 55 => &r[1..],
        _ => return None,
    };
    let hat = b[8] & 0x0F;
    let mut s = Raw { lx: b[1], ly: b[2], rx: b[3], ry: b[4], l2v: b[5], r2v: b[6], full: true, ..Default::default() };
    s.btn = [
        b[8] & 0x20 != 0, b[8] & 0x40 != 0, b[8] & 0x10 != 0, b[8] & 0x80 != 0,
        b[9] & 0x01 != 0, b[9] & 0x02 != 0, b[9] & 0x04 != 0, b[9] & 0x08 != 0,
        b[9] & 0x40 != 0, b[9] & 0x80 != 0, b[9] & 0x10 != 0, b[9] & 0x20 != 0,
        b[10] & 0x01 != 0, b[10] & 0x02 != 0,
        matches!(hat, 0 | 1 | 7), matches!(hat, 3 | 4 | 5), matches!(hat, 5 | 6 | 7), matches!(hat, 1 | 2 | 3),
    ];
    let i16at = |i: usize| i16::from_le_bytes([b[i], b[i + 1]]);
    s.gyro = [i16at(16), i16at(18), i16at(20)];
    s.accel = [i16at(22), i16at(24), i16at(26)];
    for base in [33usize, 37] {
        if b[base] & 0x80 == 0 {
            let x = b[base + 1] as u16 | (((b[base + 2] & 0x0F) as u16) << 8);
            let y = ((b[base + 2] >> 4) as u16) | ((b[base + 3] as u16) << 4);
            s.touches.push((x, y));
        }
    }
    let st = b[53];
    let level = (st & 0x0F) as u32;
    s.charging = matches!(st >> 4, 1 | 2);
    s.battery = Some((level * 10 + if s.charging { 0 } else { 5 }).min(100) as u8);
    Some(s)
}

/// Any Xbox-style (XInput) controller. Keeps the full 16-bit stick precision in `fx`.
pub fn from_xinput(w: u16, lt: u8, rt: u8, lx: i16, ly: i16, rx: i16, ry: i16) -> Raw {
    let u8x = |v: i16| ((v as i32 + 32768) >> 8) as u8;
    let u8y = |v: i16| ((32767 - v as i32).clamp(0, 65535) >> 8) as u8;
    let n = |v: i16| (v as f64 / 32767.5).clamp(-1.0, 1.0);
    let mut s = Raw { lx: u8x(lx), ly: u8y(ly), rx: u8x(rx), ry: u8y(ry), l2v: lt, r2v: rt, full: false, ..Default::default() };
    let bt = |m: u16| w & m != 0;
    s.btn = [
        bt(0x1000), bt(0x2000), bt(0x4000), bt(0x8000), bt(0x0100), bt(0x0200), lt > 30, rt > 30,
        bt(0x0040), bt(0x0080), bt(0x0020), bt(0x0010), bt(0x0400), false,
        bt(0x0001), bt(0x0002), bt(0x0004), bt(0x0008),
    ];
    s.fx = Some([n(lx), -n(ly), n(rx), -n(ry)]);
    s
}

pub fn parse_report(r: &[u8]) -> Option<Raw> {
    if r.is_empty() { return None; }
    let (b, full) = match r[0] {
        0x11 if r.len() >= 45 => (&r[2..], true),
        0x01 if r.len() >= 40 => (r, true),
        0x01 if r.len() >= 10 => (r, false),
        _ => return None,
    };
    let hat = b[5] & 0x0F;
    let mut s = Raw { lx: b[1], ly: b[2], rx: b[3], ry: b[4], l2v: b[8], r2v: b[9], full, ..Default::default() };
    s.btn = [
        b[5] & 0x20 != 0, b[5] & 0x40 != 0, b[5] & 0x10 != 0, b[5] & 0x80 != 0,
        b[6] & 0x01 != 0, b[6] & 0x02 != 0, b[6] & 0x04 != 0, b[6] & 0x08 != 0,
        b[6] & 0x40 != 0, b[6] & 0x80 != 0, b[6] & 0x10 != 0, b[6] & 0x20 != 0,
        b[7] & 0x01 != 0, b[7] & 0x02 != 0,
        matches!(hat, 0 | 1 | 7), matches!(hat, 3 | 4 | 5), matches!(hat, 5 | 6 | 7), matches!(hat, 1 | 2 | 3),
    ];
    if full {
        let i16at = |i: usize| i16::from_le_bytes([b[i], b[i + 1]]);
        s.gyro = [i16at(13), i16at(15), i16at(17)];
        s.accel = [i16at(19), i16at(21), i16at(23)];
        let bat = b[30];
        let level = (bat & 0x0F) as u32;
        s.charging = bat & 0x10 != 0;
        s.battery = Some(if s.charging { (level * 10).min(100) } else { (level * 10 + 5).min(100) } as u8);
        for base in [35usize, 39] {
            if b.len() > base + 3 && b[base] & 0x80 == 0 {
                let x = b[base + 1] as u16 | (((b[base + 2] & 0x0F) as u16) << 8);
                let y = ((b[base + 2] >> 4) as u16) | ((b[base + 3] as u16) << 4);
                s.touches.push((x, y));
            }
        }
    }
    Some(s)
}

#[inline]
pub fn norm(v: u8) -> f64 { ((v as f64 - 127.5) / 127.5).clamp(-1.0, 1.0) }

// ------------------------------------------------------------------ engine

#[derive(Clone, Copy, Debug, Default)]
pub struct Out { pub buttons: u16, pub lt: f64, pub rt: f64, pub lx: f64, pub ly: f64, pub rx: f64, pub ry: f64 }

/// Keyboard / mouse side of the output (applied with SendInput by the app).
#[derive(Clone, Debug, Default)]
pub struct Kbm { pub keys: Vec<u16>, pub mouse: u8, pub wheel: i32, pub vx: f64, pub vy: f64, pub dx: f64, pub dy: f64 }

pub struct Engine {
    pub cfg: Arc<Cfg>,
    prev_src: [bool; 18],
    dir_prev: [bool; 4],
    ps_combo_used: bool,
    ps_was: bool,
    guide_tap_until: f64,
    active: HashMap<usize, f64>,
    pub gyro_offset: [f64; 3],
    pub centers: [[f64; 2]; 2],
    last: [Option<(f64, f64)>; 2],
    pre: [Option<(f64, f64)>; 2],
    latched: [bool; 18],
    raw_prev: [bool; 18],
    wheel_next: [f64; 18],
    touch_prev: Option<(u16, u16)>,
    hr: [[Option<f64>; 2]; 2],                    // hi-res stick estimate per stick/axis
    sb: [((f64, f64), Option<((f64, f64), f64)>); 2], // anti-snapback: last output, (release direction, time)
    t_prev: f64,
    /// Set by the app: true only while a game from the single-player list is the active window.
    pub shake_ok: bool,
    rng: u64,
    shake_val: [[f64; 2]; 2],
    shake_next: [f64; 2],
}

/// Hi-res sticks: the 8-bit stick flickers between neighbouring steps; averaging that flicker recovers the
/// position *between* steps. Any real movement bigger than ~2.5 steps snaps through instantly, so fast moves
/// get zero added delay and the estimate is never more than 2.5 steps behind.
fn hires_axis(est: &mut Option<f64>, x: f64, strength: f64, dt: f64) -> f64 {
    if strength <= 0.0 { *est = None; return x; }
    let tau = 0.004 + strength * 0.026; // 4..30 ms averaging window while holding / moving slowly
    let snap = 2.5 / 127.5;
    let e = match *est { Some(e) if (x - e).abs() <= snap => e + (x - e) * (1.0 - (-dt / tau).exp()), _ => x };
    *est = Some(e);
    e
}

/// Anti-snapback: after a quick release, ignore the brief overshoot to the opposite side of center.
fn snapback(st: &mut ((f64, f64), Option<((f64, f64), f64)>), v: (f64, f64), on: bool, now: f64, dt: f64) -> (f64, f64) {
    let (last, rel) = *st;
    let m_last = last.0.hypot(last.1);
    let m = v.0.hypot(v.1);
    let mut out = v;
    if on {
        // a fast drop from a big push toward center = the stick was let go
        if m_last > 0.35 && m < m_last && (m_last - m) / dt.max(0.0005) > 6.0 {
            st.1 = Some(((last.0 / m_last, last.1 / m_last), now));
        }
        if let Some((dir, t)) = rel {
            let opposite = v.0 * dir.0 + v.1 * dir.1 < 0.0;
            if now - t > 0.06 || m > 0.6 { st.1 = None; }      // window over, or a deliberate push
            else if opposite { out = (0.0, 0.0); }             // overshoot: hold center
        }
    } else {
        st.1 = None;
    }
    st.0 = v;
    out
}

impl Engine {
    pub fn new(p: &Value) -> Self {
        Engine {
            cfg: Arc::new(Cfg::from(p)), prev_src: [false; 18], dir_prev: [false; 4], ps_combo_used: false, ps_was: false,
            guide_tap_until: 0.0, active: HashMap::new(), gyro_offset: [0.0; 3], centers: [[0.0; 2]; 2], last: [None, None], pre: [None, None],
            latched: [false; 18], raw_prev: [false; 18], wheel_next: [0.0; 18], touch_prev: None,
            hr: [[None; 2]; 2], sb: [((0.0, 0.0), None); 2], t_prev: 0.0,
            shake_ok: false, rng: 0x9E37_79B9_7F4A_7C15, shake_val: [[0.0; 2]; 2], shake_next: [0.0; 2],
        }
    }
    pub fn set_profile(&mut self, p: &Value) {
        self.cfg = Arc::new(Cfg::from(p));
        self.active.clear();
        self.latched = [false; 18];
    }

    /// Returns the Xbox output, the keyboard/mouse output and a profile-switch request (-1 / +1 / 0).
    pub fn process(&mut self, s: &Raw, now: f64) -> (Out, Kbm, i32) {
        let c = self.cfg.clone();
        let mut btn = s.btn;
        let mut cycle = 0;
        if btn[PS] {
            for (d, step) in [(16usize, -1), (17usize, 1)] {
                if btn[d] && !self.dir_prev[d - UP] {
                    self.ps_combo_used = true;
                    cycle += step;
                }
            }
            for d in UP..18 {
                self.dir_prev[d - UP] = btn[d];
                btn[d] = false;
            }
            btn[PS] = false;
        } else {
            if self.ps_was && !self.ps_combo_used {
                self.guide_tap_until = now + 0.12;
            }
            self.ps_combo_used = false;
            self.dir_prev = [false; 4];
        }
        self.ps_was = s.btn[PS];
        let guide_tap = now < self.guide_tap_until;

        let mut vals = [0f64; 18];
        for i in 0..18 { vals[i] = if btn[i] { 1.0 } else { 0.0 }; }
        vals[L2] = proc_trigger(s.l2v, &c.lt);
        vals[R2] = proc_trigger(s.r2v, &c.rt);
        // toggle mode: a press latches the button on, the next press releases it
        for i in 0..18 {
            let down = vals[i] > 0.5;
            if c.toggle[i] {
                if down && !self.raw_prev[i] { self.latched[i] = !self.latched[i]; }
                vals[i] = if self.latched[i] { 1.0 } else { 0.0 };
            }
            self.raw_prev[i] = down;
        }

        let mut blocked = [false; 18];
        for (i, m) in c.macros.iter().enumerate() {
            let pressed = vals[m.trigger] > 0.5;
            if m.block { blocked[m.trigger] = true; }
            if pressed && !self.prev_src[m.trigger] && !self.active.contains_key(&i) {
                self.active.insert(i, now);
            }
        }
        for k in 0..18 { self.prev_src[k] = vals[k] > 0.5; }

        // shift layer: while the shift button is held it does nothing itself and other buttons use their layer action
        let shift_on = c.shift_btn.map_or(false, |sbi| btn[sbi] || (sbi == L2 && s.l2v > 100) || (sbi == R2 && s.r2v > 100));
        if let Some(sbi) = c.shift_btn { blocked[sbi] = true; }
        let rate = c.turbo_rate.clamp(1.0, 30.0);
        let turbo_on = ((now * rate * 2.0) as i64) % 2 == 0;
        let (mut wb, mut lt, mut rt) = (0u16, 0f64, 0f64);
        let mut kbm = Kbm::default();
        for i in 0..18 {
            let v = vals[i];
            if v <= 0.0 || blocked[i] { self.wheel_next[i] = 0.0; continue; }
            if c.turbo[i] && !turbo_on { continue; }
            let on = v > 0.5 || ((i == L2 || i == R2) && v > 0.0);
            let tgt = if shift_on { c.shift_map[i].as_ref().unwrap_or(&c.map[i]) } else { &c.map[i] };
            match tgt {
                Tgt::Lt => lt = lt.max(v),
                Tgt::Rt => rt = rt.max(v),
                Tgt::Btn(m) if on => wb |= *m,
                Tgt::Key(k) if on && *k != 0 => kbm.keys.push(*k),
                // key combo: modifiers first, in the order written (e.g. WIN+ALT+G)
                Tgt::Keys(ks) if on => kbm.keys.extend(ks.iter().copied()),
                Tgt::Mouse(m) if on => kbm.mouse |= *m,
                Tgt::Wheel(d) if on => {
                    let d = *d;
                    // one notch on press, then repeats while held
                    if now >= self.wheel_next[i] {
                        kbm.wheel += d;
                        self.wheel_next[i] = if self.wheel_next[i] == 0.0 { now + 0.35 } else { now + 0.08 };
                    }
                }
                _ => {}
            }
        }
        // running macros
        let keys: Vec<usize> = self.active.keys().copied().collect();
        for i in keys {
            let m = &c.macros[i];
            let mut el = (now - self.active[&i]) * 1000.0;
            if el >= m.total {
                if m.repeat && vals[m.trigger] > 0.5 {
                    self.active.insert(i, now);
                    el = 0.0;
                } else {
                    self.active.remove(&i);
                    continue;
                }
            }
            let mut acc = 0.0;
            for st in &m.steps {
                if el < acc + st.ms {
                    wb |= st.mask;
                    kbm.keys.extend(st.keys.iter().copied());
                    kbm.mouse |= st.mouse;
                    if st.lt { lt = 1.0; }
                    if st.rt { rt = 1.0; }
                    break;
                }
                acc += st.ms;
            }
        }
        if guide_tap { wb |= 0x0400; }

        let dt = if self.t_prev > 0.0 { (now - self.t_prev).clamp(0.0002, 0.05) } else { 0.001 };
        self.t_prev = now;
        let (hl, hrr) = if c.swap { (c.right.hires, c.left.hires) } else { (c.left.hires, c.right.hires) };
        let v = s.fx.unwrap_or([norm(s.lx), norm(s.ly), norm(s.rx), norm(s.ry)]);
        let mut l = (hires_axis(&mut self.hr[0][0], v[0], hl, dt), hires_axis(&mut self.hr[0][1], -v[1], hl, dt));
        let mut r = (hires_axis(&mut self.hr[1][0], v[2], hrr, dt), hires_axis(&mut self.hr[1][1], -v[3], hrr, dt));
        let (mut cl, mut cr) = (self.centers[0], self.centers[1]);
        if c.swap {
            std::mem::swap(&mut l, &mut r);
            std::mem::swap(&mut cl, &mut cr);
        }
        let pl = proc_stick(l.0, l.1, &c.left, cl, self.last[0]);
        let pr = proc_stick(r.0, r.1, &c.right, cr, self.last[1]);
        // negative anti-jitter = micro-movement boost (uses the un-boosted previous sample, so it never builds up)
        let (lx, ly) = micro_boost(pl, self.pre[0], c.left.jitter);
        let (mut rx, mut ry) = micro_boost(pr, self.pre[1], c.right.jitter);
        self.pre = [Some(pl), Some(pr)];
        self.last = [Some((lx, ly)), Some((rx, ry))];
        let (lx, ly) = snapback(&mut self.sb[0], (lx, ly), c.left.snapback, now, dt);
        let (mut rx, mut ry) = snapback(&mut self.sb[1], (rx, ry), c.right.snapback, now, dt);
        // aim (ADS) sensitivity while L2 is held
        let ads = s.l2v > 60;
        let (lx, ly) = if ads && c.left.ads_sens != 1.0 { ((lx * c.left.ads_sens).clamp(-1.0, 1.0), (ly * c.left.ads_sens).clamp(-1.0, 1.0)) } else { (lx, ly) };
        if ads && c.right.ads_sens != 1.0 { rx = (rx * c.right.ads_sens).clamp(-1.0, 1.0); ry = (ry * c.right.ads_sens).clamp(-1.0, 1.0); }
        // stick shake (single-player games only - the app turns shake_ok on just for listed games)
        let (mut lx, mut ly) = (lx, ly);
        if self.shake_ok {
            for (i, sc) in [&c.left, &c.right].into_iter().enumerate() {
                if sc.shake <= 0.0 { continue; }
                if now >= self.shake_next[i] {
                    for a in 0..2 {
                        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7; self.rng ^= self.rng << 17;
                        self.shake_val[i][a] = ((self.rng >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) * sc.shake;
                    }
                    self.shake_next[i] = now + 1.0 / sc.shake_freq;
                }
                let [dx, dy] = self.shake_val[i];
                if i == 0 { lx = (lx + dx).clamp(-1.0, 1.0); ly = (ly + dy).clamp(-1.0, 1.0); }
                else { rx = (rx + dx).clamp(-1.0, 1.0); ry = (ry + dy).clamp(-1.0, 1.0); }
            }
        }

        if c.gyro_on && s.full {
            let active = match c.gyro_mode {
                0 => true,
                1 => s.l2v > 40,
                2 => s.btn[L1],
                _ => !s.touches.is_empty(),
            };
            if active {
                let yaw = (s.gyro[1] as f64 - self.gyro_offset[1]) / 16.0;
                let pitch = (s.gyro[0] as f64 - self.gyro_offset[0]) / 16.0;
                let k = c.gyro_sens / 90.0;
                let gx = -yaw * k * if c.gyro_invx { -1.0 } else { 1.0 };
                let gy = pitch * k * if c.gyro_invy { -1.0 } else { 1.0 };
                rx = (rx + gx).clamp(-1.0, 1.0);
                ry = (ry + gy).clamp(-1.0, 1.0);
            }
        }
        // stick -> mouse (that stick stops moving the virtual pad)
        let (mut lx, mut ly) = (lx, ly);
        let mut kbm = kbm;
        if c.mouse_stick != 0 {
            let (sx, sy) = if c.mouse_stick == 1 { (lx, ly) } else { (rx, ry) };
            let m = sx.hypot(sy);
            if m > 0.0 {
                let sp = m.min(1.0).powf(c.mouse_accel) * c.mouse_speed;
                kbm.vx = sx / m * sp;
                kbm.vy = -sy / m * sp;
            }
            if c.mouse_stick == 1 { lx = 0.0; ly = 0.0; } else { rx = 0.0; ry = 0.0; }
        }
        // touchpad -> trackpad
        if c.touch_mouse {
            let t = s.touches.first().copied();
            if let (Some(a), Some(p)) = (t, self.touch_prev) {
                kbm.dx = (a.0 as f64 - p.0 as f64) * 0.9 * c.touch_sens;
                kbm.dy = (a.1 as f64 - p.1 as f64) * 0.9 * c.touch_sens;
            }
            self.touch_prev = t;
        } else {
            self.touch_prev = None;
        }
        (Out { buttons: wb, lt, rt, lx, ly, rx, ry }, kbm, cycle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_python_engine() {
        let c = StickCfg::from(&stick_default());
        assert_eq!(proc_stick(0.03, 0.02, &c, [0.0; 2], None), (0.0, 0.0));
        assert!((proc_stick(0.5, 0.0, &c, [0.0; 2], None).0 - 0.4782608695652174).abs() < 1e-9);
        let cv = Curve::from(&json!({"curve":"custom","points":[[0.125,0.12],[0.25,0.25],[0.5,0.5],[0.504,0.697],[0.75,0.75],[0.931,0.346]],"y0":0,"y1":1,"smooth":true}));
        let got: Vec<f64> = [0.1, 0.3, 0.5, 0.7, 0.9, 0.97].iter().map(|x| (cv.apply(*x) * 1e4).round() / 1e4).collect();
        assert_eq!(got, vec![0.0954, 0.2559, 0.5, 0.7496, 0.3775, 0.6458]);
        let mut p = default_profile("t", "#ff2a3d");
        p["map"]["cross"] = json!("B");
        p["macros"] = json!([{"trigger":"touch","steps":"A+X 50; wait 50; Y 50"}]);
        let mut e = Engine::new(&p);
        let mut raw = Raw { lx: 128, ly: 128, rx: 128, ry: 128, r2v: 255, full: true, ..Default::default() };
        raw.btn[0] = true;
        let (o, _, _) = e.process(&raw, 1.0);
        assert_eq!((o.buttons, o.rt), (0x2000, 1.0));
        raw.btn[13] = true;
        assert_eq!(e.process(&raw, 2.0).0.buttons, 0x7000);
        assert_eq!(e.process(&raw, 2.07).0.buttons, 0x2000);
        assert_eq!(e.process(&raw, 2.12).0.buttons, 0xA000);
        // keyboard + toggle
        let mut p = default_profile("t", "#ff2a3d");
        p["map"]["square"] = json!("KEY:R");
        p["toggle"]["circle"] = json!(true);
        let mut e = Engine::new(&p);
        let mut raw = Raw { lx: 128, ly: 128, rx: 128, ry: 128, full: true, ..Default::default() };
        raw.btn[2] = true;
        raw.btn[1] = true;
        let (o, k, _) = e.process(&raw, 1.0);
        assert_eq!((k.keys.clone(), o.buttons), (vec![0x52], 0x2000));
        raw.btn = [false; 18];
        assert_eq!(e.process(&raw, 1.1).0.buttons, 0x2000); // still latched
        raw.btn[1] = true;
        e.process(&raw, 1.2);
        raw.btn[1] = false;
        assert_eq!(e.process(&raw, 1.3).0.buttons, 0); // released by second press
        // right stick -> mouse
        let mut p = default_profile("t", "#ff2a3d");
        p["mouse"]["stick"] = json!("right");
        let mut e = Engine::new(&p);
        let raw = Raw { lx: 128, ly: 128, rx: 255, ry: 128, full: true, ..Default::default() };
        let (o, k, _) = e.process(&raw, 1.0);
        assert!(k.vx > 1000.0 && o.rx == 0.0);
        // macro with a keyboard key and a mouse click
        let mut p = default_profile("t", "#ff2a3d");
        p["macros"] = json!([{"trigger":"touch","steps":"KEY:R+MOUSE_L 50; wait 20; A 30"}]);
        let mut e = Engine::new(&p);
        let mut raw = Raw { lx: 128, ly: 128, rx: 128, ry: 128, full: true, ..Default::default() };
        raw.btn[13] = true;
        let (_, k, _) = e.process(&raw, 5.0);
        assert_eq!((k.keys.clone(), k.mouse), (vec![0x52], 1));
        assert_eq!(e.process(&raw, 5.08).0.buttons, 0x1000);
        // key combo target: Win+Alt+G (Game Bar clip)
        let mut p = default_profile("t", "#ff2a3d");
        p["map"]["share"] = json!("KEYS:WIN+ALT+G");
        assert!(valid_target("KEYS:WIN+ALT+G") && !valid_target("KEYS:WIN+BOGUS"));
        let mut e = Engine::new(&p);
        let mut raw = Raw { lx: 128, ly: 128, rx: 128, ry: 128, full: true, ..Default::default() };
        raw.btn[10] = true;
        assert_eq!(e.process(&raw, 1.0).1.keys, vec![0x5B, 0xA4, 0x47]);
        // hi-res: flicker 150/151 averages to ~150.5 (between steps); a big move passes through instantly
        let mut est = None;
        let mut v = 0.0;
        for i in 0..400 { v = hires_axis(&mut est, norm(if i % 2 == 0 { 150 } else { 151 }), 1.0, 0.001); }
        assert!((v - norm(150) - 0.5 / 127.5).abs() < 0.25 / 127.5, "hires {v}");
        assert_eq!(hires_axis(&mut est, norm(200), 1.0, 0.001), norm(200));
        // anti-snapback: full right, quick release, overshoot left is held at center
        let mut st = ((0.0, 0.0), None);
        snapback(&mut st, (1.0, 0.0), true, 1.000, 0.001);
        snapback(&mut st, (0.3, 0.0), true, 1.004, 0.004);
        assert_eq!(snapback(&mut st, (-0.25, 0.0), true, 1.010, 0.006), (0.0, 0.0));
        assert_eq!(snapback(&mut st, (-0.25, 0.0), true, 1.200, 0.19), (-0.25, 0.0)); // later = real input
        // shift layer: hold L1 -> Cross becomes the R key
        let mut p = default_profile("t", "#ff2a3d");
        p["shift"] = json!({"button": "l1", "layer": {"cross": "KEY:R"}});
        let mut e = Engine::new(&p);
        let mut raw = Raw { lx: 128, ly: 128, rx: 128, ry: 128, full: true, ..Default::default() };
        raw.btn[0] = true;
        assert_eq!(e.process(&raw, 1.0).0.buttons, 0x1000);
        raw.btn[4] = true;
        let (o, k, _) = e.process(&raw, 1.1);
        assert_eq!((o.buttons, k.keys), (0, vec![0x52])); // L1 itself suppressed, Cross -> R
        // ADS: right stick halves while L2 held
        let mut p = default_profile("t", "#ff2a3d");
        p["sticks"]["right"]["ads_sens"] = json!(0.5);
        let mut e = Engine::new(&p);
        let mut raw = Raw { lx: 128, ly: 128, rx: 255, ry: 128, full: true, ..Default::default() };
        let full = e.process(&raw, 1.0).0.rx;
        raw.l2v = 200;
        assert!((e.process(&raw, 1.1).0.rx - full * 0.5).abs() < 1e-9);
        // DualSense USB report: Cross + L1 + PS, left stick right, R2 half
        let mut ds = vec![0u8; 64];
        ds[0] = 0x01; ds[1] = 255; ds[2] = 128; ds[3] = 128; ds[4] = 128; ds[6] = 128;
        ds[8] = 0x08 | 0x20; ds[9] = 0x01; ds[10] = 0x01; ds[33] = 0x80; ds[37] = 0x80; ds[53] = 0x18;
        let r = parse_dualsense(&ds).unwrap();
        assert!(r.btn[0] && r.btn[4] && r.btn[12] && !r.btn[1] && !r.btn[14] && r.lx == 255 && r.r2v == 128 && r.charging);
        // XInput: A + D-pad up, left stick full up keeps 16-bit precision
        let x = from_xinput(0x1001, 0, 255, 0, 32767, 12345, 0);
        assert!(x.btn[0] && x.btn[14] && x.btn[7] && x.ly == 0);
        assert!((x.fx.unwrap()[2] - 12345.0 / 32767.5).abs() < 1e-9);
        let mut e = Engine::new(&default_profile("t", "#ff2a3d"));
        let o = e.process(&x, 1.0).0;
        assert!(o.ly > 0.99 && (o.rx - proc_stick(12345.0 / 32767.5, 0.0, &StickCfg::from(&stick_default()), [0.0; 2], None).0).abs() < 1e-9);
        // shake: only when allowed, bounded by the amplitude
        let mut p = default_profile("t", "#ff2a3d");
        p["sticks"]["right"]["shake"] = json!(0.05);
        let mut e = Engine::new(&p);
        let rest = Raw { lx: 128, ly: 128, rx: 128, ry: 128, full: true, ..Default::default() };
        assert_eq!(e.process(&rest, 1.0).0.rx, 0.0);
        e.shake_ok = true;
        let mut moved = false;
        for i in 0..50 { let o = e.process(&rest, 2.0 + i as f64 * 0.02).0; assert!(o.rx.abs() <= 0.05 && o.ry.abs() <= 0.05); moved |= o.rx != 0.0; }
        assert!(moved);
        // negative anti-jitter boosts a tiny move, then settles on the true value when the stick stops
        assert_eq!(micro_boost((0.30, 0.0), Some((0.29, 0.0)), 0.0), (0.30, 0.0));
        let b = micro_boost((0.30, 0.0), Some((0.29, 0.0)), -0.05);
        assert!((b.0 - 0.32).abs() < 1e-9);
        assert_eq!(micro_boost((0.30, 0.0), Some((0.30, 0.0)), -0.05), (0.30, 0.0));
        assert_eq!(micro_boost((0.9, 0.0), Some((0.5, 0.0)), -0.05), (0.9, 0.0)); // big moves untouched
    }
}
