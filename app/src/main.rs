//! Edge Hub - standalone controller control center.
//! Native Rust engine + WebView2 (Chromium) window, like Spotify-style desktop apps.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;

use engine::*;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::ffi::CString;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const NO_WINDOW: u32 = 0x0800_0000;
const NEW_CONSOLE: u32 = 0x0000_0010;
const SONY: u16 = 0x054C;
const PIDS: [u16; 3] = [0x05C4, 0x09CC, 0x0BA0];
const VERSION: &str = env!("CARGO_PKG_VERSION");

// ------------------------------------------------------------------ paths

fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("EDGEHUB_DATA") {
        return PathBuf::from(d);
    }
    PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| ".".into())).join("EdgeHub")
}
fn prof_dir() -> PathBuf { data_dir().join("profiles") }
fn settings_file() -> PathBuf { data_dir().join("settings.json") }
fn safe_name(n: &str) -> String {
    let k: String = n.chars().filter(|c| c.is_alphanumeric() || " -_".contains(*c)).collect();
    let k = k.trim().to_string();
    if k.is_empty() { "Profile".into() } else { k }
}
/// Small connection log (%APPDATA%\EdgeHub\edgehub.log) so problems can be diagnosed.
fn elog(msg: &str) {
    use std::io::Write;
    let _ = fs::create_dir_all(data_dir());
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(data_dir().join("edgehub.log")) {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "[{t}] {msg}");
    }
}
fn rotate_log() {
    let p = data_dir().join("edgehub.log");
    if fs::metadata(&p).map(|m| m.len() > 256 * 1024).unwrap_or(false) { let _ = fs::rename(&p, data_dir().join("edgehub.old.log")); }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> { m.lock().unwrap_or_else(|e| e.into_inner()) }

// ------------------------------------------------------------------ shared state

type Pkt = (u8, u8, (u8, u8, u8), u8);

/// Virtual controller games see: Xbox 360 (XInput) or PlayStation 4 (DualShock 4).
enum Pad {
    X360(vigem_client::Xbox360Wired<vigem_client::Client>),
    Ds4(vigem_client::DualShock4Wired<vigem_client::Client>),
}
struct VPad { pad: Pad, notif: Option<thread::JoinHandle<()>>, last: Option<(Out, Instant)> }

/// Converts the engine's Xbox-style output into a DualShock 4 report.
fn to_ds4(o: &Out) -> vigem_client::DS4Report {
    let b = o.buttons;
    let (u, d, l, r) = (b & 0x1 != 0, b & 0x2 != 0, b & 0x4 != 0, b & 0x8 != 0);
    let hat: u16 = match (u, d, l, r) {
        (true, false, false, true) => 1, (false, true, false, true) => 3, (false, true, true, false) => 5,
        (true, false, true, false) => 7, (true, false, _, _) => 0, (_, _, false, true) => 2,
        (false, true, _, _) => 4, (_, _, true, false) => 6, _ => 8,
    };
    let mut w = hat;
    for (x, ps) in [(0x4000u16, 0x0010u16), (0x1000, 0x0020), (0x2000, 0x0040), (0x8000, 0x0080), (0x0100, 0x0100),
                    (0x0200, 0x0200), (0x0020, 0x1000), (0x0010, 0x2000), (0x0040, 0x4000), (0x0080, 0x8000)] {
        if b & x != 0 { w |= ps; }
    }
    if o.lt > 0.0 { w |= 0x0400; }
    if o.rt > 0.0 { w |= 0x0800; }
    let mut special = 0u8;
    if b & 0x0400 != 0 { special |= 0x01; }
    if b & 0x0800 != 0 { special |= 0x02; }
    let ax = |v: f64| ((v + 1.0) * 127.5).round().clamp(0.0, 255.0) as u8;
    vigem_client::DS4Report {
        thumb_lx: ax(o.lx), thumb_ly: ax(-o.ly), thumb_rx: ax(o.rx), thumb_ry: ax(-o.ry),
        buttons: w, special, trigger_l: (o.lt * 255.0) as u8, trigger_r: (o.rt * 255.0) as u8,
    }
}

impl VPad {
    fn new(sh: Arc<Shared>, ps: bool) -> Result<VPad, String> {
        let client = vigem_client::Client::connect().map_err(|e| format!("{e:?}"))?;
        if ps {
            let mut t = vigem_client::DualShock4Wired::new(client, vigem_client::TargetId::DUALSHOCK4_WIRED);
            t.plugin().map_err(|e| format!("{e:?}"))?;
            t.wait_ready().map_err(|e| format!("{e:?}"))?;
            let _ = t.update(&vigem_client::DS4Report { thumb_lx: 0x81, ..Default::default() });
            thread::sleep(Duration::from_millis(15));
            let _ = t.update(&vigem_client::DS4Report::default());
            elog("virtual PlayStation pad plugged in");
            return Ok(VPad { pad: Pad::Ds4(t), notif: None, last: None });
        }
        let before: Vec<u32> = (0..4).filter(|i| xinput_state(*i).is_some()).collect();
        let mut t = vigem_client::Xbox360Wired::new(client, vigem_client::TargetId::XBOX360_WIRED);
        t.plugin().map_err(|e| format!("{e:?}"))?;
        t.wait_ready().map_err(|e| format!("{e:?}"))?;
        // flush the pad's start-up state: a tiny change then true neutral, so games never see stale stick values
        let _ = t.update(&vigem_client::XGamepad { thumb_lx: 1, ..Default::default() });
        thread::sleep(Duration::from_millis(15));
        let _ = t.update(&vigem_client::XGamepad::default());
        // learn which XInput slot is ours so we never read our own output back (Windows can be slow to assign it)
        let mut slot = None;
        for _ in 0..40 {
            if let Ok(i) = t.get_user_index() { slot = Some(i); break; }
            let fresh: Vec<u32> = (0..4).filter(|i| xinput_state(*i).is_some() && !before.contains(i)).collect();
            if fresh.len() == 1 { slot = Some(fresh[0]); break; }
            thread::sleep(Duration::from_millis(25));
        }
        *lock(&sh.vpad_slot) = slot;
        elog(&format!("virtual Xbox pad plugged in, XInput slot {slot:?}"));
        let notif = t.request_notification().ok().map(|req| {
            req.spawn_thread(move |_, n| {
                *lock(&sh.rumble_game) = (n.large_motor, n.small_motor);
            })
        });
        Ok(VPad { pad: Pad::X360(t), notif, last: None })
    }
    fn is_ps(&self) -> bool { matches!(self.pad, Pad::Ds4(_)) }
    fn send(&mut self, o: &Out) {
        // skip identical reports (keeps 2-8 kHz controllers cheap); still refresh every 100 ms
        let now = Instant::now();
        if let Some((l, t)) = &self.last { if l == o && now.duration_since(*t) < Duration::from_millis(100) { return; } }
        self.last = Some((*o, now));
        match &mut self.pad {
            Pad::X360(t) => {
                let c = |v: f64| (v * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
                let mut btn = o.buttons;
                if btn & 0x0800 != 0 { btn = (btn & !0x0800) | 0x0020; } // touchpad click -> Back on Xbox
                let g = vigem_client::XGamepad {
                    buttons: vigem_client::XButtons(btn),
                    left_trigger: (o.lt * 255.0) as u8,
                    right_trigger: (o.rt * 255.0) as u8,
                    thumb_lx: c(o.lx), thumb_ly: c(o.ly), thumb_rx: c(o.rx), thumb_ry: c(o.ry),
                };
                let _ = t.update(&g);
            }
            Pad::Ds4(t) => { let _ = t.update(&to_ds4(o)); }
        }
    }
    fn close(mut self) {
        match &mut self.pad {
            Pad::X360(t) => { let _ = t.update(&vigem_client::XGamepad::default()); let _ = t.unplug(); }
            Pad::Ds4(t) => { let _ = t.update(&vigem_client::DS4Report::default()); let _ = t.unplug(); }
        }
        let n = self.notif.take();
        drop(self);
        if let Some(h) = n { let _ = h.join(); }
    }
}

#[derive(Default)]
struct KbmHeld { keys: Vec<u16>, mouse: u8, fx: f64, fy: f64 }

#[derive(Default, Clone)]
struct Stats { rate: u32, proc_us: f64, proc_max: f64, intervals: Vec<f32> }

struct Shared {
    settings: Mutex<Value>,
    profiles: Mutex<Vec<(String, Value)>>,
    current: Mutex<String>,
    engine: Mutex<Engine>,
    snap: Mutex<Option<(Raw, Out)>>,
    conn: Mutex<Option<Conn>>,
    devices: Mutex<Vec<DevInfo>>,
    vpad_slot: Mutex<Option<u32>>,
    fg_exe: Mutex<String>,
    shake_ok: AtomicBool,
    reopen: AtomicBool,
    writer: Mutex<Option<hidapi::HidDevice>>,
    vpad: Mutex<Option<VPad>>,
    kbm: Mutex<KbmHeld>,
    map_error: Mutex<String>,
    rumble_game: Mutex<(u8, u8)>,
    rumble_test: Mutex<(Instant, u8)>,
    flash_until: Mutex<Instant>,
    out: Mutex<(Option<Pkt>, Option<Pkt>)>,
    out_cv: Condvar,
    stats: Mutex<Stats>,
    center_samples: Mutex<Option<Vec<[f64; 4]>>>,
    gyro_samples: Mutex<Option<Vec<[i16; 3]>>>,
    recent_apps: Mutex<Vec<String>>,
    auto_base: Mutex<Option<String>>,
    running: AtomicBool,
    mapping: AtomicBool,
    released: AtomicBool,
    release_req: AtomicBool,
    no_persist: AtomicBool,
    t0: Instant,
}

impl Shared {
    fn now(&self) -> f64 { self.t0.elapsed().as_secs_f64() }

    fn new() -> Arc<Shared> {
        let _ = fs::create_dir_all(prof_dir());
        let settings: Value = fs::read_to_string(settings_file()).ok().and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| json!({"autostart_mapping": false}));
        let profiles = load_profiles();
        let cur = settings.get("current").and_then(|v| v.as_str())
            .filter(|c| profiles.iter().any(|(n, _)| n == c)).map(String::from)
            .unwrap_or_else(|| profiles[0].0.clone());
        let p = profiles.iter().find(|(n, _)| *n == cur).unwrap().1.clone();
        let mut eng = Engine::new(&p);
        for (i, k) in ["left", "right"].iter().enumerate() {
            if let Some(a) = settings["stick_center"].get(*k).and_then(|v| v.as_array()) {
                eng.centers[i] = [a.first().and_then(|x| x.as_f64()).unwrap_or(0.0), a.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0)];
            }
        }
        let now = Instant::now();
        Arc::new(Shared {
            settings: Mutex::new(settings), profiles: Mutex::new(profiles), current: Mutex::new(cur),
            engine: Mutex::new(eng), snap: Mutex::new(None), conn: Mutex::new(None), writer: Mutex::new(None),
            vpad: Mutex::new(None), kbm: Mutex::new(KbmHeld::default()), map_error: Mutex::new(String::new()),
            rumble_game: Mutex::new((0, 0)), rumble_test: Mutex::new((now, 0)), flash_until: Mutex::new(now),
            out: Mutex::new((None, None)), out_cv: Condvar::new(), stats: Mutex::new(Stats::default()),
            center_samples: Mutex::new(None), gyro_samples: Mutex::new(Some(vec![])), recent_apps: Mutex::new(vec![]),
            auto_base: Mutex::new(None),
            devices: Mutex::new(vec![]), vpad_slot: Mutex::new(None), fg_exe: Mutex::new(String::new()),
            shake_ok: AtomicBool::new(false), reopen: AtomicBool::new(false),
            running: AtomicBool::new(true), mapping: AtomicBool::new(false), released: AtomicBool::new(false),
            release_req: AtomicBool::new(false), no_persist: AtomicBool::new(false), t0: now,
        })
    }

    fn save_settings(&self) {
        if self.no_persist.load(SeqCst) { return; }
        let cur = lock(&self.current).clone();
        let mut st = lock(&self.settings);
        st["current"] = json!(cur);
        let _ = fs::create_dir_all(data_dir());
        let _ = fs::write(settings_file(), serde_json::to_string_pretty(&*st).unwrap_or_default());
    }
    fn setting_bool(&self, k: &str, d: bool) -> bool { lock(&self.settings).get(k).and_then(|v| v.as_bool()).unwrap_or(d) }
}

fn write_profile(sh: Option<&Shared>, p: &Value) {
    if sh.map_or(false, |s| s.no_persist.load(SeqCst)) { return; }
    let _ = fs::create_dir_all(prof_dir());
    let name = safe_name(p["name"].as_str().unwrap_or("Profile"));
    let _ = fs::write(prof_dir().join(format!("{name}.json")), serde_json::to_string_pretty(p).unwrap_or_default());
}

fn load_profiles() -> Vec<(String, Value)> {
    let order = |f: &str| match f { "Default.json" => 0, "FPS.json" => 1, "Racing.json" => 2, "Custom.json" => 3, _ => 9 };
    let mut files: Vec<String> = fs::read_dir(prof_dir()).map(|rd| rd.filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string()).filter(|f| f.ends_with(".json")).collect()).unwrap_or_default();
    files.sort_by_key(|f| (order(f), f.to_lowercase()));
    let mut out = vec![];
    for f in files {
        if let Ok(txt) = fs::read_to_string(prof_dir().join(&f)) {
            if let Ok(v) = serde_json::from_str::<Value>(&txt) {
                let p = deep_merge(&default_profile("Profile", "#ff2a3d"), &v);
                let n = p["name"].as_str().unwrap_or("Profile").to_string();
                if !out.iter().any(|(x, _): &(String, Value)| *x == n) { out.push((n, p)); }
            }
        }
    }
    if out.is_empty() {
        for (n, c) in [("Default", "#ff2a3d"), ("FPS", "#ff8a00"), ("Racing", "#00c2ff"), ("Custom", "#9b5cff")] {
            let mut p = default_profile(n, c);
            if n == "FPS" {
                p["sticks"]["right"]["curve"] = json!("smooth");
                p["sticks"]["left"]["inner"] = json!(0.08);
                p["triggers"]["left"]["end"] = json!(0.6);
                p["triggers"]["right"]["end"] = json!(0.6);
            }
            if n == "Racing" {
                p["sticks"]["left"]["inner"] = json!(0.03);
                p["sticks"]["left"]["curve"] = json!("precise");
            }
            write_profile(None, &p);
            out.push((n.to_string(), p));
        }
    }
    out
}

// ------------------------------------------------------------------ profiles

fn profile(sh: &Shared, name: &str) -> Option<Value> { lock(&sh.profiles).iter().find(|(n, _)| n == name).map(|(_, p)| p.clone()) }
fn names(sh: &Shared) -> Vec<String> { lock(&sh.profiles).iter().map(|(n, _)| n.clone()).collect() }

fn select_profile(sh: &Shared, name: &str) {
    if let Some(p) = profile(sh, name) {
        *lock(&sh.current) = name.to_string();
        lock(&sh.engine).set_profile(&p);
        *lock(&sh.flash_until) = Instant::now() + Duration::from_millis(600);
        if sh.setting_bool("profile_rumble", true) {
            *lock(&sh.rumble_test) = (Instant::now() + Duration::from_millis(140), 2); // short light buzz
        }
        sh.save_settings();
    }
}
fn cycle_profile(sh: &Shared, step: i32) {
    let ns = names(sh);
    let cur = lock(&sh.current).clone();
    let i = ns.iter().position(|n| *n == cur).unwrap_or(0) as i32;
    let j = (i + step).rem_euclid(ns.len() as i32) as usize;
    select_profile(sh, &ns[j]);
}
fn update_profile(sh: &Shared, p: &Value) -> Value {
    let old = lock(&sh.current).clone();
    let mut p = deep_merge(&default_profile(&old, "#ff2a3d"), p);
    let name = safe_name(p["name"].as_str().unwrap_or(&old));
    p["name"] = json!(name);
    {
        let mut ps = lock(&sh.profiles);
        if name != old {
            let _ = fs::remove_file(prof_dir().join(format!("{}.json", safe_name(&old))));
            if ps.iter().any(|(n, _)| *n == name) {
                ps.retain(|(n, _)| *n != name);
            }
        }
        if let Some(e) = ps.iter_mut().find(|(n, _)| *n == old) { *e = (name.clone(), p.clone()); } else { ps.push((name.clone(), p.clone())); }
    }
    *lock(&sh.current) = name;
    lock(&sh.engine).set_profile(&p);
    write_profile(Some(sh), &p);
    sh.save_settings();
    p
}
fn new_profile(sh: &Shared, base: Option<&str>, data: Option<&Value>) {
    let src = base.and_then(|b| profile(sh, b));
    let mut p = match (data, &src) {
        (Some(d), _) => deep_merge(&default_profile("Imported", "#ff2a3d"), d),
        (None, Some(s)) => s.clone(),
        _ => default_profile("Profile", "#ff2a3d"),
    };
    let stem = safe_name(p["name"].as_str().unwrap_or("Profile"));
    let ns = names(sh);
    let mut name = stem.clone();
    if data.is_none() || ns.contains(&name) {
        let mut i = 1;
        while ns.contains(&format!("{stem} {i}")) { i += 1; }
        name = format!("{stem} {i}");
    }
    p["name"] = json!(name);
    lock(&sh.profiles).push((name.clone(), p.clone()));
    write_profile(Some(sh), &p);
    select_profile(sh, &name);
}
fn delete_profile(sh: &Shared, name: &str) {
    {
        let mut ps = lock(&sh.profiles);
        if ps.len() <= 1 || !ps.iter().any(|(n, _)| n == name) { return; }
        ps.retain(|(n, _)| n != name);
    }
    let _ = fs::remove_file(prof_dir().join(format!("{}.json", safe_name(name))));
    if *lock(&sh.current) == name {
        let first = names(sh)[0].clone();
        select_profile(sh, &first);
    }
}

// ------------------------------------------------------------------ keyboard & mouse output

mod kbm {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    fn send(inputs: &[INPUT]) {
        if !inputs.is_empty() {
            unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
        }
    }
    pub fn key(vk: u16, up: bool) {
        let media = (0xAD..=0xB3).contains(&vk);
        let ext = matches!(vk, 0x21..=0x28 | 0x2C | 0x2D | 0x2E | 0x5B | 0xA3 | 0xA5);
        let mut ki = KEYBDINPUT { wVk: 0, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0 };
        if media {
            ki.wVk = vk;
        } else {
            ki.wScan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;
            ki.dwFlags |= KEYEVENTF_SCANCODE;
        }
        if ext { ki.dwFlags |= KEYEVENTF_EXTENDEDKEY; }
        if up { ki.dwFlags |= KEYEVENTF_KEYUP; }
        send(&[INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki } }]);
    }
    fn mi(flags: u32, dx: i32, dy: i32, data: i32) -> INPUT {
        INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: data as _, dwFlags: flags, time: 0, dwExtraInfo: 0 } } }
    }
    pub fn mouse_move(dx: i32, dy: i32) { send(&[mi(MOUSEEVENTF_MOVE, dx, dy, 0)]); }
    pub fn wheel(notches: i32) { send(&[mi(MOUSEEVENTF_WHEEL, 0, 0, notches * 120)]); }
    pub fn buttons(prev: u8, now: u8) {
        let mut v = vec![];
        for (bit, down, up, data) in [
            (1u8, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0), (2, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
            (4, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0), (8, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 1),
            (16, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 2),
        ] {
            if now & bit != 0 && prev & bit == 0 { v.push(mi(down, 0, 0, data)); }
            if now & bit == 0 && prev & bit != 0 { v.push(mi(up, 0, 0, data)); }
        }
        send(&v);
    }
}

fn apply_kbm(sh: &Shared, k: &Kbm, dt: f64) {
    let mut h = lock(&sh.kbm);
    for vkc in &k.keys { if !h.keys.contains(vkc) { kbm::key(*vkc, false); } }
    for vkc in h.keys.clone().iter().rev() { if !k.keys.contains(vkc) { kbm::key(*vkc, true); } }
    h.keys = k.keys.clone();
    if k.mouse != h.mouse { kbm::buttons(h.mouse, k.mouse); h.mouse = k.mouse; }
    if k.wheel != 0 { kbm::wheel(k.wheel); }
    h.fx += k.vx * dt + k.dx;
    h.fy += k.vy * dt + k.dy;
    let (ix, iy) = (h.fx.trunc() as i32, h.fy.trunc() as i32);
    if ix != 0 || iy != 0 {
        kbm::mouse_move(ix, iy);
        h.fx -= ix as f64;
        h.fy -= iy as f64;
    }
}
fn release_kbm(sh: &Shared) {
    let mut h = lock(&sh.kbm);
    for vkc in h.keys.drain(..).rev() { kbm::key(vkc, true); }
    if h.mouse != 0 { kbm::buttons(h.mouse, 0); h.mouse = 0; }
    h.fx = 0.0;
    h.fy = 0.0;
}

// ------------------------------------------------------------------ mapping

fn set_mapping(sh: &Arc<Shared>, on: bool) -> Value {
    if on {
        let need = lock(&sh.vpad).is_none();
        if need {
            let ps = lock(&sh.settings).get("output").and_then(|v| v.as_str()) == Some("ps");
            match VPad::new(sh.clone(), ps) {
                Ok(v) => { *lock(&sh.vpad) = Some(v); lock(&sh.map_error).clear(); }
                Err(e) => { elog(&format!("virtual pad failed: {e}")); *lock(&sh.map_error) = "missing_driver".into() }
            }
        }
        sh.mapping.store(true, SeqCst);
    } else {
        sh.mapping.store(false, SeqCst);
        if let Some(v) = lock(&sh.vpad).take() { v.close(); }
        *lock(&sh.vpad_slot) = None;
        release_kbm(sh);
        *lock(&sh.rumble_game) = (0, 0);
        lock(&sh.map_error).clear();
    }
    let pad = lock(&sh.vpad).as_ref().map(|v| if v.is_ps() { "ps" } else { "xbox" });
    json!({"mapping": sh.mapping.load(SeqCst), "xbox": pad.is_some(), "pad": pad, "error": lock(&sh.map_error).clone()})
}

fn driver_status() -> &'static str { if vigem_client::Client::connect().is_ok() { "ok" } else { "missing_driver" } }

// ------------------------------------------------------------------ light bar / rumble output

fn hsv(h: f64) -> (u8, u8, u8) {
    let i = (h * 6.0).floor();
    let f = h * 6.0 - i;
    let (p, q, t) = (0.0, 1.0 - f, f);
    let (r, g, b) = match (i as i64).rem_euclid(6) { 0 => (1.0, t, p), 1 => (q, 1.0, p), 2 => (p, 1.0, t), 3 => (p, q, 1.0), 4 => (t, p, 1.0), _ => (1.0, p, q) };
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn compute_output(sh: &Shared, s: &Raw) {
    let poll;
    let pkt: Pkt = if sh.release_req.load(SeqCst) {
        poll = 4;
        (0, 0, (0, 0, 64), poll)
    } else {
        poll = lock(&sh.settings).get("bt_poll_ms").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let c = lock(&sh.engine).cfg.clone();
        let t = (sh.now() * 30.0).floor() / 30.0; // animate at 30 fps max
        let (mut r, mut g, mut b) = c.color;
        let mut br = c.brightness;
        let batt = s.battery.unwrap_or(100) as f64 / 100.0;
        match c.light_mode {
            1 => br *= 0.12 + 0.88 * (0.5 + 0.5 * (t * c.light_speed * 2.4).sin()),
            2 => { (r, g, b) = hsv((t * c.light_speed * 0.15).fract()); }
            3 => { (r, g, b) = ((255.0 * (1.0 - batt)) as u8, (255.0 * batt) as u8, 0); }
            _ => {}
        }
        if c.light_battery && c.light_mode != 3 { (r, g, b) = ((255.0 * (1.0 - batt)) as u8, (255.0 * batt) as u8, 0); }
        let now = Instant::now();
        if now < *lock(&sh.flash_until) && ((t * 10.0) as i64) % 2 == 1 { br = 0.0; }
        // low battery warning: two red blinks every 5 s
        if sh.setting_bool("low_battery_warn", true) && !s.charging && s.battery.map_or(false, |b| b <= 15) {
            let ph = t % 5.0;
            if ph < 0.8 { (r, g, b) = (255, 0, 0); br = if ((ph * 5.0) as i64) % 2 == 0 { 1.0 } else { 0.0 }; }
        }
        let rgb = ((r as f64 * br) as u8, (g as f64 * br) as u8, (b as f64 * br) as u8);
        let (mut big, mut small) = if sh.mapping.load(SeqCst) { *lock(&sh.rumble_game) } else { (0, 0) };
        if c.rumble_swap { std::mem::swap(&mut big, &mut small); }
        big = (big as f64 * c.rumble_int) as u8;
        small = (small as f64 * c.rumble_int) as u8;
        let (until, which) = *lock(&sh.rumble_test);
        if now < until {
            let full = (255.0 * c.rumble_int) as u8;
            big = if which != 2 { full } else { 0 };
            small = if which != 1 { full } else { 0 };
        }
        (small, big, rgb, poll)
    };
    let mut o = lock(&sh.out);
    if o.1 != Some(pkt) {
        o.1 = Some(pkt);
        o.0 = Some(pkt);
        sh.out_cv.notify_one();
    }
}

/// What kind of controller we're reading.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind { Ds4, DualSense, XInput(u32) }
impl Kind {
    fn label(&self) -> &'static str { match self { Kind::Ds4 => "PS4", Kind::DualSense => "PS5", Kind::XInput(_) => "Xbox" } }
}

#[derive(Clone)]
struct Conn { id: String, bt: bool, name: String, kind: Kind }

#[derive(Clone)]
struct DevInfo { id: String, name: String, kind: Kind, bt: bool, path: Option<CString> }

enum Src { Hid(hidapi::HidDevice, Kind), X(u32) }

fn xinput_state(i: u32) -> Option<windows_sys::Win32::UI::Input::XboxController::XINPUT_STATE> {
    use windows_sys::Win32::UI::Input::XboxController::*;
    let mut st: XINPUT_STATE = unsafe { std::mem::zeroed() };
    if unsafe { XInputGetState(i, &mut st) } == 0 { Some(st) } else { None }
}

fn writer_thread(sh: Arc<Shared>) {
    let mut seq: u8 = 0;
    let mut ds_setup_for = String::new();
    while sh.running.load(SeqCst) {
        let pkt = {
            let mut o = lock(&sh.out);
            if o.0.is_none() {
                o = sh.out_cv.wait_timeout(o, Duration::from_millis(500)).unwrap_or_else(|e| e.into_inner()).0;
            }
            o.0.take()
        };
        let Some((small, big, (r, g, b), poll)) = pkt else { continue };
        let Some(c) = lock(&sh.conn).clone() else { continue };
        let crc_tail = |p: &mut Vec<u8>| {
            let mut h = crc32fast::Hasher::new();
            h.update(&[0xA2]);
            h.update(&p[..74]);
            let v = h.finalize().to_le_bytes();
            p[74..78].copy_from_slice(&v);
        };
        let data: Vec<u8> = match c.kind {
            Kind::XInput(i) => {
                use windows_sys::Win32::UI::Input::XboxController::*;
                let v = XINPUT_VIBRATION { wLeftMotorSpeed: big as u16 * 257, wRightMotorSpeed: small as u16 * 257 };
                unsafe { XInputSetState(i, &v) };
                continue;
            }
            Kind::Ds4 if !c.bt => {
                let mut p = vec![0u8; 32];
                p[..11].copy_from_slice(&[0x05, 0x07, 0x04, 0x00, small, big, r, g, b, 0, 0]);
                p
            }
            Kind::Ds4 => {
                let mut p = vec![0u8; 78];
                p[0] = 0x11; p[1] = 0xC0 | (poll & 0x0F); p[2] = 0x20; p[3] = 0x07; p[4] = 0x04;
                p[6] = small; p[7] = big; p[8] = r; p[9] = g; p[10] = b;
                crc_tail(&mut p);
                p
            }
            Kind::DualSense => {
                // PS5 output: rumble (compatible mode) + light bar; the first packet also releases the default light animation
                let mut cm = [0u8; 47];
                cm[0] = 0x03; cm[1] = 0x04; cm[2] = small; cm[3] = big;
                if ds_setup_for != c.id { cm[38] = 0x02; cm[41] = 0x02; ds_setup_for = c.id.clone(); }
                cm[44] = r; cm[45] = g; cm[46] = b;
                if !c.bt {
                    let mut p = vec![0u8; 63];
                    p[0] = 0x02;
                    p[1..48].copy_from_slice(&cm);
                    p
                } else {
                    let mut p = vec![0u8; 78];
                    seq = (seq + 1) & 0x0F;
                    p[0] = 0x31; p[1] = seq << 4; p[2] = 0x10;
                    p[3..50].copy_from_slice(&cm);
                    crc_tail(&mut p);
                    p
                }
            }
        };
        if let Some(dev) = lock(&sh.writer).as_ref() { let _ = dev.write(&data); }
    }
}

// ------------------------------------------------------------------ input loop

/// Every supported controller that's connected right now (PS4 + clones, PS5 / Edge, any XInput pad).
fn scan_devices(api: &mut hidapi::HidApi, sh: &Shared) -> Vec<DevInfo> {
    let _ = api.refresh_devices();
    let mut v = vec![];
    for d in api.device_list() {
        if d.vendor_id() != SONY || d.usage_page() != 1 || d.usage() != 5 { continue; }
        let kind = match d.product_id() { 0x05C4 | 0x09CC | 0x0BA0 => Kind::Ds4, 0x0CE6 | 0x0DF2 => Kind::DualSense, _ => continue };
        let up = d.path().to_string_lossy().to_uppercase();
        // never read our own virtual PlayStation pad (bus-enumerated: no USB interface tag, not Bluetooth)
        if up.contains("&REV_") && !up.contains("&MI_") && !up.contains("00001124") { continue; }
        let def = match d.product_id() { 0x0DF2 => "DualSense Edge", 0x0CE6 => "DualSense", _ => "Wireless Controller" };
        let name = d.product_string().filter(|s| !s.trim().is_empty()).unwrap_or(def).to_string();
        v.push(DevInfo { id: d.path().to_string_lossy().to_string(), name, kind, bt: up.contains("00001124"), path: Some(d.path().to_owned()) });
    }
    v.sort_by_key(|d| d.bt);
    let skip = *lock(&sh.vpad_slot);
    for i in 0..4u32 {
        if Some(i) == skip { continue; } // our own virtual Xbox pad
        if xinput_state(i).is_some() {
            v.push(DevInfo { id: format!("xinput:{i}"), name: format!("Xbox controller (player {})", i + 1), kind: Kind::XInput(i), bt: false, path: None });
        }
    }
    v
}

fn open_controller(api: &mut hidapi::HidApi, sh: &Shared) -> Option<Src> {
    let mut devs = scan_devices(api, sh);
    *lock(&sh.devices) = devs.clone();
    let pref = lock(&sh.settings).get("device").and_then(|v| v.as_str()).map(String::from);
    if let Some(p) = pref {
        if let Some(i) = devs.iter().position(|d| d.id == p) { let d = devs.remove(i); devs.insert(0, d); }
    }
    for d in devs {
        let conn = Conn { id: d.id.clone(), bt: d.bt, name: d.name.clone(), kind: d.kind };
        if let Kind::XInput(i) = d.kind {
            *lock(&sh.writer) = None;
            *lock(&sh.conn) = Some(conn);
            lock(&sh.out).1 = None;
            return Some(Src::X(i));
        }
        let Some(path) = d.path.clone() else { continue };
        let Ok(dev) = api.open_path(&path) else { continue };
        let mut buf = [0u8; 128];
        if !matches!(dev.read_timeout(&mut buf, 300), Ok(n) if n > 0) { continue; }
        if d.bt {
            let mut fr = [0u8; 41];
            fr[0] = 0x05;
            let _ = dev.get_feature_report(&mut fr); // switch Bluetooth into full report mode
        }
        *lock(&sh.writer) = api.open_path(&path).ok();
        *lock(&sh.conn) = Some(conn);
        *lock(&sh.gyro_samples) = Some(vec![]);
        lock(&sh.out).1 = None;
        return Some(Src::Hid(dev, d.kind));
    }
    None
}

fn input_thread(sh: Arc<Shared>) {
    unsafe {
        windows_sys::Win32::Media::timeBeginPeriod(1);
        use windows_sys::Win32::System::Threading::*;
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
    }
    let mut api = loop {
        match hidapi::HidApi::new() { Ok(a) => break a, Err(_) => thread::sleep(Duration::from_secs(1)) }
    };
    let mut src: Option<Src> = None;
    let mut buf = [0u8; 128];
    let (mut count, mut rate_t) = (0u32, Instant::now());
    let mut last_report: Option<Instant> = None;
    let mut last_scan = Instant::now();
    let mut ring: std::collections::VecDeque<f32> = std::collections::VecDeque::with_capacity(512);
    let mut last_data = Instant::now();
    let mut last_pkt: u32 = u32::MAX;
    let mut fresh = true;
    let drop_conn = |sh: &Shared| { *lock(&sh.writer) = None; *lock(&sh.conn) = None; *lock(&sh.snap) = None; };
    while sh.running.load(SeqCst) {
        if sh.released.load(SeqCst) {
            src = None;
            drop_conn(&sh);
            thread::sleep(Duration::from_millis(250));
            continue;
        }
        if sh.reopen.swap(false, SeqCst) { src = None; drop_conn(&sh); }
        if src.is_none() {
            *lock(&sh.snap) = None;
            src = open_controller(&mut api, &sh);
            last_scan = Instant::now();
            if src.is_none() {
                *lock(&sh.conn) = None;
                thread::sleep(Duration::from_secs(1));
                continue;
            }
            if let Some(c) = lock(&sh.conn).as_ref() { elog(&format!("connected: {} {} ({})", c.kind.label(), c.name, if c.bt { "Bluetooth" } else { "USB/XInput" })); }
            last_report = None;
            last_data = Instant::now();
            last_pkt = u32::MAX;
        }
        if last_scan.elapsed() > Duration::from_secs(3) {
            let devs = scan_devices(&mut api, &sh); // keeps the controller picker up to date
            // automatic mode: a PlayStation controller always wins over an XInput pad
            let auto = lock(&sh.settings).get("device").and_then(|v| v.as_str()).map_or(true, |s| s.is_empty());
            let switch = auto && matches!(src, Some(Src::X(_))) && devs.iter().any(|d| !matches!(d.kind, Kind::XInput(_)));
            *lock(&sh.devices) = devs;
            last_scan = Instant::now();
            if switch { elog("PlayStation controller appeared: switching to it"); src = None; drop_conn(&sh); continue; }
        }
        let got: Result<Option<Raw>, ()> = match src.as_ref().unwrap() {
            Src::Hid(dev, kind) => match dev.read_timeout(&mut buf, 200) {
                Ok(0) => Ok(None),
                Ok(n) => Ok(if *kind == Kind::DualSense { parse_dualsense(&buf[..n]) } else { parse_report(&buf[..n]) }),
                Err(_) => Err(()),
            },
            Src::X(i) => {
                // ~1000 Hz polling; controllers that update faster (2-8 kHz) get a tight 8 kHz poll
                if lock(&sh.stats).rate > 1500 {
                    let t = Instant::now();
                    while t.elapsed() < Duration::from_micros(110) { std::hint::spin_loop(); }
                } else {
                    thread::sleep(Duration::from_micros(900));
                }
                match xinput_state(*i) {
                    Some(st) => {
                        fresh = st.dwPacketNumber != last_pkt;
                        last_pkt = st.dwPacketNumber;
                        let g = st.Gamepad;
                        Ok(Some(from_xinput(g.wButtons as u16, g.bLeftTrigger, g.bRightTrigger, g.sThumbLX, g.sThumbLY, g.sThumbRX, g.sThumbRY)))
                    }
                    None => Err(()),
                }
            }
        };
        if let Some(Src::Hid(..)) = src { fresh = true; }
        let s = match got {
            Ok(Some(s)) => { last_data = Instant::now(); s }
            Ok(None) => {
                // watchdog: a controller that goes silent (sleep/resume, USB hiccup) gets reconnected
                if matches!(src, Some(Src::Hid(..))) && last_data.elapsed() > Duration::from_millis(1500) {
                    elog("controller went silent for 1.5 s: reconnecting");
                    src = None;
                    drop_conn(&sh);
                }
                continue;
            }
            Err(_) => { elog("controller disconnected"); src = None; drop_conn(&sh); continue; }
        };
        let t0 = Instant::now();
        let dt = last_report.map(|l| (t0 - l).as_secs_f64()).unwrap_or(0.0);
        if fresh {
            if dt > 0.0 {
                if ring.len() >= 480 { ring.pop_front(); }
                ring.push_back((dt * 1e6) as f32);
            }
            last_report = Some(t0);
        }

        // calibrations
        {
            let mut cs = lock(&sh.center_samples);
            if let Some(v) = cs.as_mut() {
                let f = s.fx.unwrap_or([norm(s.lx), norm(s.ly), norm(s.rx), norm(s.ry)]);
                v.push([f[0], -f[1], f[2], -f[3]]);
                if v.len() >= 120 {
                    let k = v.len() as f64;
                    let a: Vec<f64> = (0..4).map(|i| v.iter().map(|x| x[i]).sum::<f64>() / k).collect();
                    lock(&sh.engine).centers = [[a[0], a[1]], [a[2], a[3]]];
                    lock(&sh.settings)["stick_center"] = json!({"left": [a[0], a[1]], "right": [a[2], a[3]]});
                    *cs = None;
                    drop(cs);
                    sh.save_settings();
                }
            }
        }
        if s.full {
            let mut gs = lock(&sh.gyro_samples);
            if let Some(v) = gs.as_mut() {
                v.push(s.gyro);
                if v.len() >= 150 {
                    let k = v.len() as f64;
                    let a: Vec<f64> = (0..3).map(|i| v.iter().map(|x| x[i] as f64).sum::<f64>() / k).collect();
                    lock(&sh.engine).gyro_offset = [a[0], a[1], a[2]];
                    *gs = None;
                }
            }
        }

        let (o, k, cycle) = {
            let mut e = lock(&sh.engine);
            e.shake_ok = sh.shake_ok.load(SeqCst);
            e.process(&s, sh.now())
        };
        if cycle != 0 { cycle_profile(&sh, cycle); }
        if sh.mapping.load(SeqCst) {
            if let Some(v) = lock(&sh.vpad).as_mut() { v.send(&o); }
            apply_kbm(&sh, &k, dt.min(0.05));
        }
        compute_output(&sh, &s);
        let us = t0.elapsed().as_secs_f64() * 1e6;
        *lock(&sh.snap) = Some((s, o));
        if fresh { count += 1; }
        let mut st = lock(&sh.stats);
        st.proc_us = st.proc_us * 0.95 + us * 0.05;
        st.proc_max = (st.proc_max * 0.999).max(us);
        if rate_t.elapsed() >= Duration::from_secs(1) {
            st.rate = (count as f64 / rate_t.elapsed().as_secs_f64()).round() as u32;
            st.intervals = ring.iter().copied().collect();
            count = 0;
            rate_t = Instant::now();
        }
    }
}

// ------------------------------------------------------------------ foreground game watcher (auto profile switching)

fn foreground_exe() -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() { return None; }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() { return None; }
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 { return None; }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        full.rsplit('\\').next().map(|s| s.to_lowercase())
    }
}

fn watcher_thread(sh: Arc<Shared>) {
    while sh.running.load(SeqCst) {
        thread::sleep(Duration::from_millis(700));
        let Some(exe) = foreground_exe() else { continue };
        *lock(&sh.fg_exe) = exe.clone();
        // stick shake only runs while a game from the single-player list is the active window
        let sp = lock(&sh.settings).get("shake_games").and_then(|v| v.as_array()).map_or(false,
            |g| g.iter().any(|x| x.as_str().map_or(false, |s| s.eq_ignore_ascii_case(&exe))));
        sh.shake_ok.store(sp, SeqCst);
        if exe == "edgehub.exe" { continue; }
        {
            let mut r = lock(&sh.recent_apps);
            r.retain(|x| *x != exe);
            r.insert(0, exe.clone());
            r.truncate(15);
        }
        if !sh.setting_bool("auto_switch", true) { continue; }
        let hit = lock(&sh.profiles).iter().find(|(_, p)| p["games"].as_array()
            .map_or(false, |g| g.iter().any(|x| x.as_str().map_or(false, |s| s.eq_ignore_ascii_case(&exe))))).map(|(n, _)| n.clone());
        let cur = lock(&sh.current).clone();
        match hit {
            Some(n) if n != cur => {
                let mut base = lock(&sh.auto_base);
                if base.is_none() { *base = Some(cur); }
                drop(base);
                select_profile(&sh, &n);
            }
            None => {
                if let Some(b) = lock(&sh.auto_base).take() {
                    if profile(&sh, &b).is_some() { select_profile(&sh, &b); }
                }
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------------ system status & restore

const SYSTEM_SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$devs = Get-PnpDevice -PresentOnly | Where-Object { $_.InstanceId -match 'VID_054C&PID_(09CC|05C4|0BA0)|VID&0002054C_PID&(09CC|05C4|0BA0)' }
$list = foreach ($d in $devs) {
  $id = $d.InstanceId
  $p = Get-PnpDeviceProperty -InstanceId $id -KeyName DEVPKEY_Device_Service,DEVPKEY_Device_DriverProvider,DEVPKEY_Device_UpperFilters,DEVPKEY_Device_LowerFilters,DEVPKEY_Device_DriverInfPath
  function G($k) { ($p | Where-Object { $_.KeyName -eq $k }).Data }
  [pscustomobject]@{ name = $d.FriendlyName; id = $id; status = "$($d.Status)"; service = "$(G 'DEVPKEY_Device_Service')";
    provider = "$(G 'DEVPKEY_Device_DriverProvider')"; inf = "$(G 'DEVPKEY_Device_DriverInfPath')";
    upper = (@(G 'DEVPKEY_Device_UpperFilters') | Where-Object { $_ }) -join ','; lower = (@(G 'DEVPKEY_Device_LowerFilters') | Where-Object { $_ }) -join ',' }
}
$un = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' |
  Where-Object { $_.DisplayName -match 'ViGEm|HidHide' } | Select-Object DisplayName, UninstallString, PSChildName
[pscustomobject]@{ devices = @($list); uninstall = @($un);
  vigem = (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Services\ViGEmBus');
  hidhide = (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Services\HidHide');
  hidusbf = (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Services\hidusbf') } | ConvertTo-Json -Depth 4 -Compress
"#;

fn as_list(v: &Value) -> Vec<Value> {
    match v { Value::Array(a) => a.clone(), Value::Null => vec![], x => vec![x.clone()] }
}

fn system_status() -> Value {
    let out = Command::new("powershell").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", SYSTEM_SCRIPT])
        .creation_flags(NO_WINDOW).output();
    let mut data: Value = match out {
        Ok(o) => serde_json::from_slice(String::from_utf8_lossy(&o.stdout).trim().as_bytes()).unwrap_or(json!({})),
        Err(e) => return json!({"error": e.to_string(), "devices": [], "uninstall": []}),
    };
    let mut devs = vec![];
    for d in as_list(&data["devices"]) {
        let filters: Vec<String> = format!("{},{}", d["upper"].as_str().unwrap_or(""), d["lower"].as_str().unwrap_or(""))
            .split(',').filter(|x| !x.is_empty()).map(String::from).collect();
        let prov = d["provider"].as_str().filter(|p| !p.is_empty()).unwrap_or("Microsoft");
        let id = d["id"].as_str().unwrap_or("").to_uppercase();
        let mut d = d.clone();
        d["standard"] = json!(prov == "Microsoft" && filters.is_empty());
        d["usb_parent"] = json!(id.starts_with("USB\\") && !id.contains("&MI_"));
        d["filters"] = json!(filters);
        devs.push(d);
    }
    data["all_standard"] = if devs.is_empty() { Value::Null } else { json!(devs.iter().all(|d| d["standard"] == json!(true))) };
    data["devices"] = json!(devs);
    data["uninstall"] = json!(as_list(&data["uninstall"]));
    data
}

fn run_elevated(lines: &[String]) -> Option<String> {
    let tmp = std::env::temp_dir();
    let ps1 = tmp.join("edgehub_restore.ps1");
    let log = tmp.join("edgehub_restore.log");
    let mut body = vec!["$ErrorActionPreference = 'Continue'".to_string(), format!("Start-Transcript -Path '{}' -Force | Out-Null", log.display())];
    body.extend(lines.iter().cloned());
    body.push("Stop-Transcript | Out-Null".into());
    let _ = fs::write(&ps1, format!("\u{feff}{}", body.join("\r\n")));
    let _ = fs::remove_file(&log);
    let cmd = format!("Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','\"{}\"'", ps1.display());
    let st = Command::new("powershell").args(["-NoProfile", "-Command", &cmd]).creation_flags(NO_WINDOW).status().ok()?;
    if !st.success() { return None; }
    Some(fs::read_to_string(&log).unwrap_or_default())
}

fn release(sh: &Arc<Shared>) {
    set_mapping(sh, false);
    {
        let mut st = lock(&sh.settings);
        st["autostart_mapping"] = json!(false);
        if let Some(o) = st.as_object_mut() { o.remove("bt_poll_ms"); }
    }
    sh.save_settings();
    sh.release_req.store(true, SeqCst);
    lock(&sh.out).1 = None;
    thread::sleep(Duration::from_millis(500));
    sh.released.store(true, SeqCst);
    thread::sleep(Duration::from_millis(400));
    sh.release_req.store(false, SeqCst);
}

fn restore(sh: &Arc<Shared>, o: &Value) -> Value {
    let opt = |k: &str| o.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    let mut log: Vec<(String, String)> = vec![];
    let mut add = |k: &str, t: &str| log.push((k.into(), t.into()));
    release(sh);
    add("ok", "Remapping turned off and the virtual controller removed");
    add("ok", "Controller released: rumble off, light bar back to plain blue, Bluetooth poll rate back to normal");
    add("ok", "Edge Hub won't start remapping on its own any more");
    let st = system_status();
    let mut lines = vec![];
    if hidhide_installed() {
        lines.push(format!("& '{HH_CLI}' --cloak-off"));
        lock(&sh.settings)["hidhide_on"] = json!(false);
        add("ok", "HidHide turned off: games can see your real controller again");
    }
    let mut did = vec![];
    if opt("reset_driver") {
        let parents: Vec<String> = as_list(&st["devices"]).iter().filter(|d| d["usb_parent"] == json!(true))
            .filter_map(|d| d["id"].as_str().map(String::from)).collect();
        for p in &parents { lines.push(format!("pnputil /remove-device \"{p}\" /subtree")); }
        if parents.is_empty() {
            add("warn", "No USB controller found to reinstall. Plug it in with a cable and try again. Over Bluetooth, remove and re-pair it in Windows Bluetooth settings instead.");
        } else {
            lines.push("pnputil /scan-devices".into());
            did.push("reset_driver");
        }
    }
    for (key, label) in [("uninstall_vigem", "ViGEm"), ("uninstall_hidhide", "HidHide")] {
        if !opt(key) { continue; }
        for u in as_list(&st["uninstall"]) {
            if !u["DisplayName"].as_str().unwrap_or("").to_lowercase().contains(&label.to_lowercase()) { continue; }
            let code = u["PSChildName"].as_str().unwrap_or("");
            if code.starts_with('{') {
                lines.push(format!("Start-Process msiexec -ArgumentList '/x {code} /qn /norestart' -Wait"));
                did.push(key);
            } else if let Some(us) = u["UninstallString"].as_str() {
                lines.push(format!("Start-Process cmd -ArgumentList '/c \"{}\"' -Wait", us.replace('\'', "''")));
                did.push(key);
            }
        }
        if !did.contains(&key) { add("warn", &format!("{label} isn't installed, so there was nothing to uninstall")); }
    }
    if !lines.is_empty() {
        match run_elevated(&lines) {
            None => add("bad", "Windows permission prompt was declined, so drivers were not changed"),
            Some(_) => {
                if did.contains(&"reset_driver") { add("ok", "Controller removed and re-detected: Windows reinstalled its standard driver"); }
                if did.contains(&"uninstall_vigem") { add("ok", "ViGEmBus (virtual controller driver) uninstalled"); }
                if did.contains(&"uninstall_hidhide") { add("ok", "HidHide uninstalled"); }
            }
        }
        thread::sleep(Duration::from_millis(2500));
    }
    if opt("delete_settings") {
        sh.no_persist.store(true, SeqCst);
        let _ = fs::remove_dir_all(data_dir());
        add("ok", "Edge Hub profiles and settings deleted");
    }
    let after = system_status();
    if after["hidusbf"] == json!(true) {
        add("warn", "A USB overclock tool (hidusbf) is still installed. Open its setup and click Uninstall to remove it completely.");
    }
    json!({"log": log.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>(), "status": after})
}

// ------------------------------------------------------------------ OBS overlay server (localhost only)

const PAD_JS: &str = include_str!("../../ui/pad.js");
const OVERLAY_HTML: &str = include_str!("../../ui/overlay.html");

fn overlay_port(sh: &Shared) -> u16 {
    lock(&sh.settings).get("overlay_port").and_then(|v| v.as_u64()).filter(|p| *p >= 1024 && *p < 65535).unwrap_or(8650) as u16
}
fn overlay_url(sh: &Shared) -> String { format!("http://127.0.0.1:{}/overlay", overlay_port(sh)) }

/// Compact live state for the overlay (about 60 updates a second).
fn overlay_state(sh: &Shared) -> String {
    let col = { let c = lock(&sh.engine).cfg.color; format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2) };
    match lock(&sh.snap).clone() {
        Some((s, _)) => {
            let bits: u32 = s.btn.iter().enumerate().map(|(i, b)| (*b as u32) << i).sum();
            json!({"c": true, "b": bits, "lx": s.lx, "ly": s.ly, "rx": s.rx, "ry": s.ry, "l2": s.l2v, "r2": s.r2v,
                   "t": s.touches.iter().map(|t| [t.0, t.1]).collect::<Vec<_>>(), "col": col}).to_string()
        }
        None => json!({"c": false, "b": 0, "lx": 128, "ly": 128, "rx": 128, "ry": 128, "l2": 0, "r2": 0, "t": [], "col": col}).to_string(),
    }
}

fn overlay_client(sh: Arc<Shared>, mut st: std::net::TcpStream) {
    use std::io::{Read, Write};
    let _ = st.set_read_timeout(Some(Duration::from_secs(3)));
    let mut buf = [0u8; 4096];
    let mut n = 0;
    while n < buf.len() {
        match st.read(&mut buf[n..]) { Ok(0) | Err(_) => break, Ok(k) => n += k }
        if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") { break; }
    }
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req.split_whitespace().nth(1).unwrap_or("/").split('?').next().unwrap_or("/").to_string();
    let send = |st: &mut std::net::TcpStream, ctype: &str, body: &[u8]| {
        let head = format!("HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n", body.len());
        let _ = st.write_all(head.as_bytes());
        let _ = st.write_all(body);
    };
    match path.as_str() {
        "/" | "/overlay" | "/overlay.html" => send(&mut st, "text/html; charset=utf-8", OVERLAY_HTML.as_bytes()),
        "/pad.js" => send(&mut st, "application/javascript; charset=utf-8", PAD_JS.as_bytes()),
        "/config" => {
            let viz = lock(&sh.settings).get("viz").cloned().unwrap_or(json!({}));
            send(&mut st, "application/json", json!({"viz": viz}).to_string().as_bytes());
        }
        "/events" => {
            let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: keep-alive\r\n\r\n";
            if st.write_all(head.as_bytes()).is_err() { return; }
            let _ = st.set_nodelay(true);
            while sh.running.load(SeqCst) {
                if st.write_all(format!("data: {}\n\n", overlay_state(&sh)).as_bytes()).is_err() { break; }
                thread::sleep(Duration::from_millis(16));
            }
        }
        _ => {
            let _ = st.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    }
}

fn overlay_server(sh: Arc<Shared>) {
    let port = overlay_port(&sh);
    match std::net::TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => {
            elog(&format!("OBS overlay ready at http://127.0.0.1:{port}/overlay"));
            for c in l.incoming().flatten() {
                let s = sh.clone();
                thread::spawn(move || overlay_client(s, c));
            }
        }
        Err(e) => elog(&format!("OBS overlay could not start on port {port}: {e}")),
    }
}

// ------------------------------------------------------------------ HidHide (hide the real controller from games)

const HH_CLI: &str = r"C:\Program Files\Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe";
fn hidhide_installed() -> bool { std::path::Path::new(HH_CLI).exists() }
/// "\\?\HID#VID_054C&PID_09CC&MI_03#7&1689be04&0&0000#{guid}" -> "HID\VID_054C&PID_09CC&MI_03\7&1689BE04&0&0000"
fn instance_from_path(p: &str) -> Option<String> {
    let parts: Vec<&str> = p.trim_start_matches(r"\\?\").split('#').collect();
    if parts.len() < 3 { return None; }
    Some(format!("{}\\{}\\{}", parts[0], parts[1], parts[2]).to_uppercase())
}
fn hidhide_status(sh: &Shared) -> Value {
    let dev = lock(&sh.conn).as_ref().filter(|c| !matches!(c.kind, Kind::XInput(_))).and_then(|c| instance_from_path(&c.id));
    json!({"installed": hidhide_installed(), "on": sh.setting_bool("hidhide_on", false), "device": dev})
}
fn hidhide_set(sh: &Shared, on: bool) -> Value {
    if !hidhide_installed() {
        let _ = Command::new("cmd").args(["/c", "title Installing HidHide && winget install --id Nefarius.HidHide -e --accept-source-agreements --accept-package-agreements && echo. && echo Done. Restart your PC, then turn the switch on again in Edge Hub. && pause"])
            .creation_flags(NEW_CONSOLE).spawn();
        return json!({"ok": false, "installing": true});
    }
    let q = |s: &str| format!("'{}'", s.replace('\'', "''"));
    let mut lines = vec![];
    if on {
        let mut exes = vec![PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Programs\\EdgeHub\\EdgeHub.exe")];
        if let Ok(me) = std::env::current_exe() { exes.push(me); }
        for e in exes { lines.push(format!("& {} --app-reg {}", q(HH_CLI), q(&e.display().to_string()))); }
        let Some(dev) = hidhide_status(sh)["device"].as_str().map(String::from) else {
            return json!({"ok": false, "error": "Connect a PlayStation controller first"});
        };
        lines.push(format!("& {} --dev-hide {}", q(HH_CLI), q(&dev)));
        lines.push(format!("& {} --cloak-on", q(HH_CLI)));
    } else {
        lines.push(format!("& {} --cloak-off", q(HH_CLI)));
    }
    match run_elevated(&lines) {
        None => json!({"ok": false, "error": "The Windows permission prompt was declined"}),
        Some(_) => {
            lock(&sh.settings)["hidhide_on"] = json!(on);
            sh.save_settings();
            elog(if on { "HidHide: real controller hidden from games" } else { "HidHide: real controller visible again" });
            json!({"ok": true})
        }
    }
}

fn set_autostart(on: bool) {
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
    let _ = if on {
        Command::new("reg").args(["add", key, "/v", "EdgeHub", "/t", "REG_SZ", "/d", &format!("\"{exe}\" --minimized"), "/f"]).creation_flags(NO_WINDOW).status()
    } else {
        Command::new("reg").args(["delete", key, "/v", "EdgeHub", "/f"]).creation_flags(NO_WINDOW).status()
    };
}

/// Only one Edge Hub at a time; a second launch brings the first one to the front.
fn first_instance() -> bool {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let name: Vec<u16> = "Local\\EdgeHubSingleInstance\0".encode_utf16().collect();
    let h = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if h.is_null() || unsafe { GetLastError() } != ERROR_ALREADY_EXISTS { return true; }
    // ask the running copy to show its window (works even when it's hidden in the tray)
    let ev: Vec<u16> = SHOW_EVENT.encode_utf16().collect();
    unsafe {
        use windows_sys::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};
        let e = OpenEventW(EVENT_MODIFY_STATE, 0, ev.as_ptr());
        if !e.is_null() { SetEvent(e); }
    }
    false
}
const SHOW_EVENT: &str = "Local\\EdgeHubShowWindow\0";

/// Waits for "show yourself" signals from later launches.
fn show_signal_thread(proxy: tao::event_loop::EventLoopProxy<UserEvent>) {
    use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
    let ev: Vec<u16> = SHOW_EVENT.encode_utf16().collect();
    let h = unsafe { CreateEventW(std::ptr::null(), 0, 0, ev.as_ptr()) };
    if h.is_null() { return; }
    let h = h as usize;
    thread::spawn(move || {
        loop {
            if unsafe { WaitForSingleObject(h as _, u32::MAX) } != 0 { break; }
            if proxy.send_event(UserEvent::Show).is_err() { break; }
        }
    });
}

// ------------------------------------------------------------------ UI state + API

fn state_json(sh: &Shared) -> Value {
    let conn = lock(&sh.conn).clone();
    let snap = lock(&sh.snap).clone();
    let released = sh.released.load(SeqCst);
    let mut v = match (conn, snap) {
        (Some(Conn { bt, name, kind, .. }), Some((s, o))) => {
            let st = lock(&sh.stats).clone();
            let iv = &st.intervals;
            let (mut mn, mut mx, mut sum) = (f32::MAX, 0f32, 0f64);
            for x in iv { mn = mn.min(*x); mx = mx.max(*x); sum += *x as f64; }
            let avg = if iv.is_empty() { 0.0 } else { sum / iv.len() as f64 };
            let jit = if iv.is_empty() { 0.0 } else { (iv.iter().map(|x| (*x as f64 - avg).powi(2)).sum::<f64>() / iv.len() as f64).sqrt() };
            let eng = lock(&sh.engine);
            let btn: serde_json::Map<String, Value> = SOURCES.iter().enumerate().map(|(i, k)| (k.to_string(), json!(s.btn[i]))).collect();
            json!({
                "connected": true, "conn": match kind { Kind::XInput(_) => "XInput", _ if bt => "Bluetooth", _ => "USB" },
                "name": name, "kind": kind.label(), "full": s.full,
                "rate": st.rate, "proc_us": (st.proc_us * 10.0).round() / 10.0, "proc_us_max": (st.proc_max * 10.0).round() / 10.0,
                "battery": s.battery, "charging": s.charging, "lx": s.lx, "ly": s.ly, "rx": s.rx, "ry": s.ry, "l2v": s.l2v, "r2v": s.r2v,
                "btn": btn, "gyro": s.gyro, "accel": s.accel, "touches": s.touches.iter().map(|t| [t.0, t.1]).collect::<Vec<_>>(),
                "out": {"buttons": o.buttons, "lt": o.lt, "rt": o.rt, "lx": o.lx, "ly": o.ly, "rx": o.rx, "ry": o.ry},
                "calibrating": lock(&sh.gyro_samples).is_some(), "centering": lock(&sh.center_samples).is_some(),
                "centers": {"left": eng.centers[0], "right": eng.centers[1]},
                "poll": {"min": if iv.is_empty() { 0.0 } else { mn as f64 }, "max": mx, "avg": avg, "jitter": jit,
                         "recent": iv.iter().rev().take(160).rev().collect::<Vec<_>>()},
            })
        }
        _ => json!({"connected": false}),
    };
    v["released"] = json!(released);
    v["mapping"] = json!(sh.mapping.load(SeqCst));
    let pad = lock(&sh.vpad).as_ref().map(|p| if p.is_ps() { "ps" } else { "xbox" });
    v["xbox"] = json!(pad.is_some());
    v["pad"] = json!(pad);
    v["map_error"] = json!(lock(&sh.map_error).clone());
    v["current"] = json!(lock(&sh.current).clone());
    v["auto"] = json!(lock(&sh.auto_base).is_some());
    v["shake_ok"] = json!(sh.shake_ok.load(SeqCst));
    v["fg"] = json!(lock(&sh.fg_exe).clone());
    v["hidhide"] = json!(sh.setting_bool("hidhide_on", false));
    let rg = *lock(&sh.rumble_game);
    v["rumble_game"] = json!([rg.0, rg.1]);
    v
}

fn get_init(sh: &Shared) -> Value {
    let cur = lock(&sh.current).clone();
    json!({
        "profiles": names(sh), "current": cur, "profile": profile(sh, &cur), "sources": SOURCES, "targets": TARGETS,
        "settings": lock(&sh.settings).clone(), "driver": driver_status(), "data_dir": data_dir().display().to_string(),
        "native": true, "version": VERSION, "recent_apps": lock(&sh.recent_apps).clone(), "devices": devices_json(sh),
        "overlay_url": overlay_url(sh),
    })
}

fn devices_json(sh: &Shared) -> Value {
    let cur = lock(&sh.conn).as_ref().map(|c| c.id.clone());
    json!(lock(&sh.devices).iter().map(|d| json!({
        "id": d.id, "name": d.name, "kind": d.kind.label(),
        "conn": match d.kind { Kind::XInput(_) => "XInput", _ if d.bt => "Bluetooth", _ => "USB" },
        "active": Some(&d.id) == cur.as_ref(),
    })).collect::<Vec<_>>())
}

enum UserEvent { Eval(String), Quit, Close, Show, ToggleMap, Tick, Drag, Min, MaxToggle, Resize(u8) }

fn api_call(sh: &Arc<Shared>, m: &str, a: &[Value], proxy: &tao::event_loop::EventLoopProxy<UserEvent>) -> Result<Value, String> {
    let arg = |i: usize| a.get(i).cloned().unwrap_or(Value::Null);
    let s0 = || arg(0).as_str().unwrap_or("").to_string();
    Ok(match m {
        "get_init" => get_init(sh),
        "save_profile" => {
            let p = update_profile(sh, &arg(0));
            json!({"profiles": names(sh), "current": lock(&sh.current).clone(), "profile": p})
        }
        "select_profile" => { *lock(&sh.auto_base) = None; select_profile(sh, &s0()); get_init(sh) }
        "new_profile" => { let b = arg(0); new_profile(sh, b.as_str(), None); get_init(sh) }
        "import_profile" => { new_profile(sh, None, Some(&arg(0))); get_init(sh) }
        "delete_profile" => { delete_profile(sh, &s0()); get_init(sh) }
        "reset_profile" => {
            let cur = lock(&sh.current).clone();
            let col = profile(sh, &cur).and_then(|p| p["color"].as_str().map(String::from)).unwrap_or("#ff2a3d".into());
            let p = update_profile(sh, &default_profile(&cur, &col));
            json!({"profiles": names(sh), "current": cur, "profile": p})
        }
        "set_mapping" => {
            let on = arg(0).as_bool().unwrap_or(false);
            let r = set_mapping(sh, on);
            if on { lock(&sh.settings)["autostart_mapping"] = json!(true); sh.save_settings(); }
            r
        }
        "set_setting" => {
            let k = s0();
            lock(&sh.settings)[k.as_str()] = arg(1);
            sh.save_settings();
            if k == "bt_poll_ms" { lock(&sh.out).1 = None; }
            if k == "device" { sh.reopen.store(true, SeqCst); }
            if k == "start_with_windows" { set_autostart(arg(1).as_bool().unwrap_or(false)); }
            if k == "output" && sh.mapping.load(SeqCst) {
                // switch the virtual controller type live
                if let Some(v) = lock(&sh.vpad).take() { v.close(); }
                set_mapping(sh, true);
            }
            Value::Null
        }
        "test_rumble" => {
            let w = match s0().as_str() { "big" => 1, "small" => 2, _ => 0 };
            *lock(&sh.rumble_test) = (Instant::now() + Duration::from_millis(600), w);
            Value::Null
        }
        "calibrate_gyro" => { *lock(&sh.gyro_samples) = Some(vec![]); Value::Null }
        "calibrate_sticks" => { *lock(&sh.center_samples) = Some(vec![]); Value::Null }
        "clear_stick_center" => {
            lock(&sh.engine).centers = [[0.0; 2]; 2];
            lock(&sh.settings)["stick_center"] = json!({"left": [0, 0], "right": [0, 0]});
            sh.save_settings();
            Value::Null
        }
        "driver_status" => json!(driver_status()),
        "install_driver" => {
            let _ = Command::new("cmd").args(["/c", "title Installing ViGEmBus && winget install --id ViGEm.ViGEmBus -e --accept-source-agreements --accept-package-agreements && echo. && echo Done - restart Edge Hub. && pause"])
                .creation_flags(NEW_CONSOLE).spawn();
            json!(true)
        }
        "open_data_dir" => { let _ = fs::create_dir_all(data_dir()); let _ = Command::new("explorer").arg(data_dir()).spawn(); Value::Null }
        "recent_apps" => json!(lock(&sh.recent_apps).clone()),
        "list_devices" => devices_json(sh),
        "open_url" => {
            let u = s0();
            if u.starts_with("http://127.0.0.1:") { let _ = Command::new("cmd").args(["/c", "start", "", &u]).creation_flags(NO_WINDOW).spawn(); }
            Value::Null
        }
        "hidhide_status" => hidhide_status(sh),
        "hidhide_set" => hidhide_set(sh, arg(0).as_bool().unwrap_or(false)),
        "read_log" => json!(fs::read_to_string(data_dir().join("edgehub.log")).unwrap_or_default()
            .lines().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")),
        "system_status" => system_status(),
        "restore" => restore(sh, &arg(0)),
        "release" => { release(sh); Value::Null }
        "resume" => { sh.released.store(false, SeqCst); lock(&sh.out).1 = None; Value::Null }
        "quit" => { let _ = proxy.send_event(UserEvent::Quit); Value::Null }
        "win_close" => { let _ = proxy.send_event(UserEvent::Close); Value::Null }
        "win_drag" => { let _ = proxy.send_event(UserEvent::Drag); Value::Null }
        "win_min" => { let _ = proxy.send_event(UserEvent::Min); Value::Null }
        "win_max" => { let _ = proxy.send_event(UserEvent::MaxToggle); Value::Null }
        "win_resize" => { let _ = proxy.send_event(UserEvent::Resize(arg(0).as_u64().unwrap_or(0) as u8)); Value::Null }
        _ => return Err(format!("unknown call {m}")),
    })
}

const SHIM: &str = r#"
(() => {
  window.__EH_NATIVE = true;
  const pend = {}; let id = 0; let st = {connected: false};
  window.__eh_state = s => { st = s; };
  window.__eh_reply = (i, ok, v) => { const p = pend[i]; if (!p) return; delete pend[i]; ok ? p.res(v) : p.rej(new Error(v)); };
  const call = (m, args) => new Promise((res, rej) => { const i = ++id; pend[i] = {res, rej}; window.ipc.postMessage(JSON.stringify({id: i, m, args})); });
  const api = new Proxy({}, { get: (_, m) => m === 'get_state' ? () => Promise.resolve(st) : (...a) => call(m, a) });
  window.pywebview = { api };
  document.addEventListener('DOMContentLoaded', () => setTimeout(() => window.dispatchEvent(new Event('pywebviewready')), 0));
  document.addEventListener('contextmenu', e => { if (!e.target.closest('canvas,input,textarea')) e.preventDefault(); });
})();
"#;

// ------------------------------------------------------------------ main

fn main() {
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::platform::windows::WindowBuilderExtWindows;
    use tao::window::{Icon, ResizeDirection, WindowBuilder};
    use wry::{http::Response, WebContext, WebViewBuilder};

    let test_mode = std::env::args().any(|a| a.starts_with("--") && a != "--minimized");
    let minimized = std::env::args().any(|a| a == "--minimized");
    if !test_mode && !first_instance() { return; }
    let sh = Shared::new();
    if !test_mode {
        rotate_log();
        elog(&format!("Edge Hub {VERSION} starting"));
        // create the virtual pad BEFORE looking for controllers, so it can never be mistaken for yours
        if sh.setting_bool("autostart_mapping", false) { set_mapping(&sh, true); }
    }
    { let s = sh.clone(); thread::Builder::new().name("input".into()).spawn(move || input_thread(s)).unwrap(); }
    { let s = sh.clone(); thread::spawn(move || writer_thread(s)); }
    { let s = sh.clone(); thread::spawn(move || watcher_thread(s)); }
    if !test_mode { let s = sh.clone(); thread::spawn(move || overlay_server(s)); }
    if std::env::args().any(|a| a == "--coldtest") {
        // same order as a normal launch: threads already running, then autostart remapping
        if sh.setting_bool("autostart_mapping", false) { set_mapping(&sh, true); }
        for ms in [500u64, 1000, 2000, 3500] {
            thread::sleep(Duration::from_millis(ms.saturating_sub(0)));
            let c = lock(&sh.conn).clone();
            println!("  t+{ms}ms conn={:?} pad={:?} slot={:?} rate={}", c.map(|c| (c.kind.label(), c.name, c.bt)),
                lock(&sh.vpad).as_ref().map(|v| v.is_ps()), *lock(&sh.vpad_slot), lock(&sh.stats).rate);
        }
        println!("  devices seen: {}", devices_json(&sh));
        set_mapping(&sh, false);
        return;
    }
    if std::env::args().any(|a| a == "--shaketest") {
        // shake must stay off unless the active window is on the single-player list
        let cur = lock(&sh.current).clone();
        let mut p = profile(&sh, &cur).unwrap();
        p["sticks"]["right"]["shake"] = json!(0.05);
        update_profile(&sh, &p);
        let spread = |sh: &Shared| {
            let (mut lo, mut hi) = (f64::MAX, f64::MIN);
            for _ in 0..60 { if let Some((_, o)) = lock(&sh.snap).clone() { lo = lo.min(o.ry); hi = hi.max(o.ry); } thread::sleep(Duration::from_millis(10)); }
            hi - lo
        };
        thread::sleep(Duration::from_millis(1600));
        let fg = lock(&sh.fg_exe).clone();
        println!("active window: {fg}  gate={}  right-stick spread without list: {:.4}", sh.shake_ok.load(SeqCst), spread(&sh));
        lock(&sh.settings)["shake_games"] = json!([fg]);
        thread::sleep(Duration::from_millis(1600));
        println!("after adding {fg}: gate={}  right-stick spread: {:.4} (max allowed 0.10)", sh.shake_ok.load(SeqCst), spread(&sh));
        return;
    }
    if std::env::args().any(|a| a == "--devices") {
        // lists every supported controller, then reads each one for a moment
        thread::sleep(Duration::from_millis(3500));
        println!("{}", serde_json::to_string_pretty(&devices_json(&sh)).unwrap_or_default());
        let list = lock(&sh.devices).clone();
        for d in list {
            lock(&sh.settings)["device"] = json!(d.id);
            sh.reopen.store(true, SeqCst);
            for ms in [100u64, 300, 600, 1200, 2500] {
                thread::sleep(Duration::from_millis(ms));
                println!("  +{ms}ms connected={} conn={}", state_json(&sh)["connected"], lock(&sh.conn).is_some());
            }
            let st = state_json(&sh);
            println!("{} [{}]: connected={} kind={} rate={} battery={} L=({},{}) gyro={}",
                d.name, d.kind.label(), st["connected"], st["kind"], st["rate"], st["battery"], st["lx"], st["ly"], st["gyro"]);
        }
        if let Some(o) = lock(&sh.settings).as_object_mut() { o.remove("device"); }
        return;
    }
    if std::env::args().any(|a| a == "--keytest") {
        // sends a harmless combo (Ctrl+Shift+F23) through the real keyboard path, then releases it
        let mut k = Kbm::default();
        k.keys = vec![vk("CTRL"), vk("SHIFT"), vk("F23")];
        apply_kbm(&sh, &k, 0.0);
        thread::sleep(Duration::from_millis(80));
        apply_kbm(&sh, &Kbm::default(), 0.0);
        println!("keytest sent");
        return;
    }
    if std::env::args().any(|a| a == "--selftest") {
        // headless check: read the controller for 3 s, try the virtual pad, print the state
        thread::sleep(Duration::from_secs(3));
        let st = state_json(&sh);
        println!("connected={} conn={} rate={}Hz proc_us={} battery={} poll={}",
            st["connected"], st["conn"], st["rate"], st["proc_us"], st["battery"], st["poll"]["avg"]);
        println!("driver={} profiles={:?} current={}", driver_status(), names(&sh), st["current"]);
        let m = set_mapping(&sh, true);
        println!("mapping={m}");
        let hold = std::env::var("EDGEHUB_HOLD").ok().and_then(|v| v.parse().ok()).unwrap_or(500);
        thread::sleep(Duration::from_millis(1000));
        let st = state_json(&sh);
        println!("raw L=({},{}) R=({},{}) out={}", st["lx"], st["ly"], st["rx"], st["ry"], st["out"]);
        if std::env::var("EDGEHUB_PATTERN").is_ok() {
            sh.released.store(true, SeqCst); // stop live input so only the pattern is sent
            thread::sleep(Duration::from_millis(400));
            let o = Out { buttons: 0x1000 | 0x0800 | 0x0009, lt: 0.0, rt: 1.0, lx: 0.5, ly: 0.0, rx: 0.0, ry: -1.0 };
            for _ in 0..40 { if let Some(v) = lock(&sh.vpad).as_mut() { v.send(&o); } thread::sleep(Duration::from_millis(50)); }
            println!("pattern sent");
        }
        thread::sleep(Duration::from_millis(hold));
        set_mapping(&sh, false);
        sh.running.store(false, SeqCst);
        return;
    }
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let icon = Icon::from_rgba(include_bytes!("../icon64.rgba").to_vec(), 64, 64).ok();
    let window = WindowBuilder::new()
        .with_title("Edge Hub")
        .with_inner_size(LogicalSize::new(1260.0, 820.0))
        .with_min_inner_size(LogicalSize::new(1000.0, 680.0))
        .with_decorations(false)
        .with_undecorated_shadow(true)
        .with_window_icon(icon.clone())
        .with_taskbar_icon(icon)
        .with_visible(!minimized)
        .build(&event_loop)
        .expect("window");

    let wv_dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into())).join("EdgeHub").join("WebView2");
    let mut ctx = WebContext::new(Some(wv_dir));
    let html: &'static str = include_str!("../../ui/index.html");
    let ipc_sh = sh.clone();
    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new_with_web_context(&mut ctx)
        .with_custom_protocol("edgehub".into(), move |_id, req| {
            if req.uri().path().ends_with("/pad.js") {
                return Response::builder().header("Content-Type", "application/javascript; charset=utf-8").body(Cow::Borrowed(PAD_JS.as_bytes())).unwrap();
            }
            Response::builder().header("Content-Type", "text/html; charset=utf-8").body(Cow::Borrowed(html.as_bytes())).unwrap()
        })
        .with_url("http://edgehub.localhost/")
        .with_initialization_script(SHIM)
        .with_background_color((11, 11, 15, 255))
        .with_devtools(cfg!(debug_assertions))
        .with_ipc_handler(move |req| {
            let msg: Value = serde_json::from_str(req.body()).unwrap_or(Value::Null);
            let id = msg["id"].as_i64().unwrap_or(0);
            let m = msg["m"].as_str().unwrap_or("").to_string();
            let args = msg["args"].as_array().cloned().unwrap_or_default();
            let (sh, proxy) = (ipc_sh.clone(), ipc_proxy.clone());
            thread::spawn(move || {
                let (ok, v) = match api_call(&sh, &m, &args, &proxy) { Ok(v) => (true, v), Err(e) => (false, json!(e)) };
                let _ = proxy.send_event(UserEvent::Eval(format!("window.__eh_reply({id},{ok},{v})")));
            });
        })
        .build(&window)
        .expect("webview");

    {
        let (s, p) = (sh.clone(), proxy.clone());
        thread::spawn(move || {
            let mut n = 0u32;
            while s.running.load(SeqCst) {
                let js = format!("window.__eh_state&&window.__eh_state({})", state_json(&s));
                if p.send_event(UserEvent::Eval(js)).is_err() { break; }
                n += 1;
                if n % 120 == 0 { let _ = p.send_event(UserEvent::Tick); }
                thread::sleep(Duration::from_millis(16));
            }
        });
    }

    show_signal_thread(proxy.clone());

    // ---- system tray: keeps Edge Hub (and your remaps) running when the window is closed
    use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    let m_open = MenuItem::new("Open Edge Hub", true, None);
    let m_map = CheckMenuItem::new("Remapping", true, sh.mapping.load(SeqCst), None);
    let m_quit = MenuItem::new("Quit Edge Hub", true, None);
    let tray_menu = Menu::new();
    let _ = tray_menu.append_items(&[&m_open, &m_map, &PredefinedMenuItem::separator(), &m_quit]);
    let tray = tray_icon::Icon::from_rgba(include_bytes!("../icon64.rgba").to_vec(), 64, 64).ok().and_then(|ic|
        TrayIconBuilder::new().with_menu(Box::new(tray_menu)).with_tooltip("Edge Hub").with_icon(ic).build().ok());
    {
        let p = proxy.clone();
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            let show = matches!(e, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } | TrayIconEvent::DoubleClick { .. });
            if show { let _ = p.send_event(UserEvent::Show); }
        }));
        let p = proxy.clone();
        let (open_id, map_id, quit_id) = (m_open.id().clone(), m_map.id().clone(), m_quit.id().clone());
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let ev = if e.id == open_id { UserEvent::Show } else if e.id == map_id { UserEvent::ToggleMap } else if e.id == quit_id { UserEvent::Quit } else { return };
            let _ = p.send_event(ev);
        }));
    }

    let shut = sh.clone();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let quit = |cf: &mut ControlFlow| {
            set_mapping(&shut, false);
            shut.running.store(false, SeqCst);
            *cf = ControlFlow::Exit;
        };
        let close_to_tray = shut.setting_bool("close_to_tray", true) && tray.is_some();
        match event {
            Event::UserEvent(UserEvent::Eval(js)) => { let _ = webview.evaluate_script(&js); }
            Event::UserEvent(UserEvent::Quit) => quit(control_flow),
            Event::UserEvent(UserEvent::Close) => { if close_to_tray { window.set_visible(false); } else { quit(control_flow); } }
            Event::UserEvent(UserEvent::Show) => { window.set_visible(true); window.set_minimized(false); window.set_focus(); }
            Event::UserEvent(UserEvent::ToggleMap) => {
                let on = !shut.mapping.load(SeqCst);
                set_mapping(&shut, on);
                lock(&shut.settings)["autostart_mapping"] = json!(on);
                shut.save_settings();
                m_map.set_checked(shut.mapping.load(SeqCst));
            }
            Event::UserEvent(UserEvent::Tick) => {
                m_map.set_checked(shut.mapping.load(SeqCst));
                if let Some(t) = &tray {
                    let st = state_json(&shut);
                    let tip = if st["connected"] == json!(true) {
                        format!("Edge Hub - {} {} - {} Hz{}{}", st["kind"].as_str().unwrap_or(""), st["conn"].as_str().unwrap_or(""), st["rate"],
                            st["battery"].as_u64().map(|b| format!(" - {b}%")).unwrap_or_default(),
                            if shut.mapping.load(SeqCst) { " - Remapping ON" } else { "" })
                    } else { "Edge Hub - no controller".to_string() };
                    let _ = t.set_tooltip(Some(tip));
                }
            }
            Event::UserEvent(UserEvent::Drag) => { let _ = window.drag_window(); }
            Event::UserEvent(UserEvent::Min) => window.set_minimized(true),
            Event::UserEvent(UserEvent::MaxToggle) => window.set_maximized(!window.is_maximized()),
            Event::UserEvent(UserEvent::Resize(d)) => {
                let dir = match d {
                    1 => ResizeDirection::North, 2 => ResizeDirection::NorthEast, 3 => ResizeDirection::East,
                    4 => ResizeDirection::SouthEast, 5 => ResizeDirection::South, 6 => ResizeDirection::SouthWest,
                    7 => ResizeDirection::West, _ => ResizeDirection::NorthWest,
                };
                let _ = window.drag_resize_window(dir);
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                if close_to_tray { window.set_visible(false); } else { quit(control_flow); }
            }
            _ => {}
        }
    });
}
