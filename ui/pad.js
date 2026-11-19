/* Edge Hub controller visualizer: shared by the app and the OBS overlay.
   Original artwork: models, versions and skins are drawn in SVG so every part can animate. */
(function () {
  'use strict';

  // ---------------------------------------------------------------- models
  const MODELS = {
    ds4:  { name: 'PS4 style',  versions: { v2: 'V2: light bar in touchpad', v1: 'V1: classic' }, def: 'v2' },
    ds5:  { name: 'PS5 style',  versions: { std: 'Standard', edge: 'Edge: Fn buttons' }, def: 'std' },
    xbox: { name: 'Xbox style', versions: { series: 'Series: Share button', one: 'One' }, def: 'series' },
  };

  // ---------------------------------------------------------------- skins
  const SKIN_DEFAULTS = {
    body: '#1b1b1f', center: null, grip: null, gripTex: false, texture: 'grain', camo: null,
    btn: '#1f1f24', finish: 'matte', dpad: null, symbols: 'classic',
    touch: null, touchFinish: 'matte', stick: '#18181b', ring: null,
    trig: null, trigFinish: null, accent: '#ff2a3d', home: null,
  };
  const SKINS = [
    { id: 'hexred', name: 'Red Chrome', tag: 'like yours', body: '#161619', gripTex: true, btn: '#c8102e', finish: 'chrome', symbols: 'none',
      touch: '#b0122a', touchFinish: 'brushed', ring: '#d6142e', trig: '#c8102e', accent: '#ff2a3d' },
    { id: 'jet', name: 'Jet Black', body: '#1b1b1f', btn: '#202026', symbols: 'classic', accent: '#3d7bff' },
    { id: 'glacier', name: 'Glacier White', body: '#e4e7ec', texture: 'none', btn: '#f2f3f6', finish: 'gloss', symbols: 'dark', touch: '#eceef2', stick: '#e6e8ec', trig: '#e4e7ec', accent: '#3d7bff' },
    { id: 'twotone', name: 'Two-Tone White', body: '#eef0f3', texture: 'none', center: '#141417', btn: '#16161a', symbols: 'white', touch: '#eef0f3', stick: '#141417', trig: '#eef0f3', accent: '#4c8dff' },
    { id: 'cosmic', name: 'Cosmic Red', body: '#7a1320', center: '#101013', btn: '#16161a', symbols: 'white', touch: '#7a1320', stick: '#121215', trig: '#7a1320', accent: '#ff3b4e' },
    { id: 'galactic', name: 'Galactic Purple', body: '#3a2470', center: '#0f0f13', btn: '#16161a', symbols: 'white', touch: '#3a2470', stick: '#121215', trig: '#3a2470', accent: '#9b5cff' },
    { id: 'gold', name: 'Gold Chrome', body: '#151517', gripTex: true, btn: '#d4a017', finish: 'chrome', symbols: 'none', touch: '#c49317', touchFinish: 'brushed', ring: '#d4a017', trig: '#d4a017', accent: '#ffd25a' },
    { id: 'ice', name: 'Ice Blue Chrome', body: '#141518', gripTex: true, btn: '#2a8bff', finish: 'chrome', symbols: 'none', touch: '#1f6fd6', touchFinish: 'brushed', ring: '#3b97ff', trig: '#2a8bff', accent: '#5cb8ff' },
    { id: 'toxic', name: 'Toxic Green', body: '#121314', gripTex: true, btn: '#2fcf4f', finish: 'chrome', symbols: 'none', touch: '#22a83e', touchFinish: 'brushed', ring: '#3be060', trig: '#2fcf4f', accent: '#4dff7a' },
    { id: 'silver', name: 'Silver Chrome', body: '#18181b', gripTex: true, btn: '#b9bec6', finish: 'chrome', symbols: 'none', touch: '#9aa0a8', touchFinish: 'brushed', ring: '#c8ccd2', trig: '#b9bec6', accent: '#e8ecf2' },
    { id: 'stealth', name: 'Stealth', body: '#0f0f11', texture: 'grain', gripTex: true, btn: '#141417', symbols: 'none', touch: '#111114', stick: '#0e0e10', trig: '#131316', accent: '#ffffff' },
    { id: 'carbon', name: 'Carbon Fiber', body: '#1c1c20', texture: 'carbon', gripTex: true, btn: '#c8102e', finish: 'gloss', symbols: 'white', touch: '#1a1a1e', ring: '#c8102e', trig: '#c8102e', accent: '#ff2a3d' },
    { id: 'arctic', name: 'Arctic Camo', body: '#c3cad2', texture: 'camo', camo: ['#e6eaee', '#98a2ad', '#5d6672'], btn: '#2b2f35', symbols: 'white', touch: '#d5dae0', stick: '#25282d', trig: '#2b2f35', accent: '#4fb4ff' },
    { id: 'forest', name: 'Forest Camo', body: '#4f5b3a', texture: 'camo', camo: ['#6b7a4b', '#3a4429', '#8c8a5c'], btn: '#22251c', symbols: 'white', touch: '#4a5536', stick: '#1d2018', trig: '#22251c', accent: '#9be15d' },
    { id: 'shock', name: 'Shock Blue', body: '#1d5fd0', texture: 'grain', btn: '#14161b', symbols: 'classic', touch: '#1b57bf', stick: '#121418', trig: '#14161b', accent: '#5aa8ff' },
    { id: 'robot', name: 'Robot White', body: '#eceef1', texture: 'none', btn: '#2a2b30', symbols: 'classic', touch: '#e4e6ea', stick: '#2a2b30', trig: '#e2e4e8', accent: '#4bd264' },
    { id: 'sunset', name: 'Sunset Orange', body: '#e2571b', texture: 'grain', btn: '#16161a', symbols: 'white', touch: '#c94b15', stick: '#141417', trig: '#16161a', accent: '#ffa047' },
    { id: 'pink', name: 'Nova Pink', body: '#f0a6c4', texture: 'none', center: '#f7f2f5', btn: '#f7f2f5', finish: 'gloss', symbols: 'dark', touch: '#f7f2f5', stick: '#f2e8ee', trig: '#f0a6c4', accent: '#ff5fa8' },
  ];
  const DEF = { model: 'ds4', version: 'v2', skin: 'hexred', custom: {},
    overlay: { scale: 1, bg: 'transparent', meters: true, glow: true, shadow: true } };

  // ---------------------------------------------------------------- color helpers
  const hx = h => { h = String(h || '#000').replace('#', ''); if (h.length === 3) h = h.split('').map(c => c + c).join(''); const n = parseInt(h, 16) || 0; return [n >> 16 & 255, n >> 8 & 255, n & 255]; };
  const toHex = (r, g, b) => '#' + [r, g, b].map(v => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('');
  const mix = (a, b, t) => { const A = hx(a), B = hx(b); return toHex(A[0] + (B[0] - A[0]) * t, A[1] + (B[1] - A[1]) * t, A[2] + (B[2] - A[2]) * t); };
  const L = (c, t) => mix(c, '#ffffff', t), D = (c, t) => mix(c, '#000000', t);
  const lum = c => { const [r, g, b] = hx(c); return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255; };

  function resolve(viz) {
    viz = Object.assign({}, DEF, viz || {});
    viz.overlay = Object.assign({}, DEF.overlay, (viz && viz.overlay) || {});
    if (!MODELS[viz.model]) viz.model = 'ds4';
    if (!MODELS[viz.model].versions[viz.version]) viz.version = MODELS[viz.model].def;
    const base = SKINS.find(s => s.id === viz.skin) || SKINS[0];
    const k = Object.assign({}, SKIN_DEFAULTS, base, viz.custom || {});
    k.grip = k.grip || D(k.body, lum(k.body) > 0.5 ? 0.12 : 0.25);
    k.touch = k.touch || L(k.body, 0.04);
    k.trig = k.trig || k.btn;
    k.trigFinish = k.trigFinish || k.finish;
    k.dpad = k.dpad || k.btn;
    k.home = k.home || D(k.stick, 0.1);
    return { viz, skin: k };
  }

  // ---------------------------------------------------------------- gradients / patterns
  function fill(id, name, c, finish) {
    if (finish === 'chrome') return `<linearGradient id="${id}-${name}" x1="0" y1="0" x2="0.25" y2="1">
      <stop offset="0" stop-color="${L(c, .7)}"/><stop offset=".2" stop-color="${L(c, .28)}"/><stop offset=".46" stop-color="${c}"/>
      <stop offset=".54" stop-color="${D(c, .42)}"/><stop offset=".8" stop-color="${L(c, .1)}"/><stop offset="1" stop-color="${D(c, .55)}"/></linearGradient>`;
    if (finish === 'gloss') return `<radialGradient id="${id}-${name}" cx=".35" cy=".25" r=".85">
      <stop offset="0" stop-color="${L(c, .5)}"/><stop offset=".3" stop-color="${L(c, .1)}"/><stop offset=".65" stop-color="${c}"/><stop offset="1" stop-color="${D(c, .45)}"/></radialGradient>`;
    if (finish === 'brushed') return `<linearGradient id="${id}-${name}" x1="0" y1="0" x2="1" y2="0.3">
      <stop offset="0" stop-color="${D(c, .25)}"/><stop offset=".28" stop-color="${L(c, .22)}"/><stop offset=".5" stop-color="${c}"/>
      <stop offset=".72" stop-color="${L(c, .16)}"/><stop offset="1" stop-color="${D(c, .3)}"/></linearGradient>`;
    return `<radialGradient id="${id}-${name}" cx=".38" cy=".3" r=".85">
      <stop offset="0" stop-color="${L(c, .14)}"/><stop offset=".55" stop-color="${c}"/><stop offset="1" stop-color="${D(c, .45)}"/></radialGradient>`;
  }
  function textures(id, k) {
    const camo = k.camo || [L(k.body, .2), D(k.body, .3), D(k.body, .55)];
    return `
      <pattern id="${id}-grain" width="5" height="5" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r=".6" fill="#fff" opacity=".04"/><circle cx="3.5" cy="3.2" r=".5" fill="#000" opacity=".22"/></pattern>
      <pattern id="${id}-carbon" width="10" height="10" patternUnits="userSpaceOnUse">
        <rect width="10" height="10" fill="${D(k.body, .35)}"/><rect width="5" height="5" fill="${L(k.body, .12)}"/><rect x="5" y="5" width="5" height="5" fill="${L(k.body, .12)}"/>
        <rect width="5" height="2.5" fill="#fff" opacity=".06"/><rect x="5" y="5" width="5" height="2.5" fill="#fff" opacity=".06"/></pattern>
      <pattern id="${id}-camo" width="160" height="160" patternUnits="userSpaceOnUse">
        <rect width="160" height="160" fill="${k.body}"/>
        <path d="M10 20 C40 0 70 30 60 50 C45 75 5 60 10 20Z M95 15 C125 5 150 25 140 45 C130 60 100 55 95 15Z M30 95 C55 80 85 95 80 120 C70 145 25 135 30 95Z M105 90 C140 85 160 110 150 135 C135 150 100 130 105 90Z" fill="${camo[0]}"/>
        <path d="M60 60 C80 50 100 70 90 85 C80 100 55 90 60 60Z M0 130 C20 120 35 140 25 155 C15 165 -5 150 0 130Z M120 55 C135 50 150 65 140 75 C130 85 115 75 120 55Z" fill="${camo[1]}"/>
        <path d="M40 40 C50 35 60 45 55 52 C48 58 38 50 40 40Z M85 130 C100 125 110 140 100 150 C90 155 80 145 85 130Z M140 5 C150 0 160 10 155 18 C148 25 138 15 140 5Z" fill="${camo[2]}"/></pattern>
      <pattern id="${id}-gripdots" width="7" height="7" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
        <rect width="7" height="7" fill="${k.grip}"/><rect x="1" y="1" width="3.2" height="3.2" rx=".8" fill="${L(k.grip, .1)}"/><rect x="1.4" y="1.4" width="2.4" height="2.4" rx=".6" fill="${D(k.grip, .35)}"/></pattern>
      <pattern id="${id}-hair" width="200" height="3" patternUnits="userSpaceOnUse"><rect width="200" height="1" fill="#fff" opacity=".07"/><rect y="2" width="200" height="1" fill="#000" opacity=".1"/></pattern>`;
  }

  // ---------------------------------------------------------------- geometry (measured proportions)
  const f1 = v => +v.toFixed(1);
  // smooth closed curve through points (Catmull-Rom -> cubic Bezier)
  function smooth(pts) {
    const n = pts.length;
    let d = `M${f1(pts[0][0])} ${f1(pts[0][1])}`;
    for (let i = 0; i < n; i++) {
      const p0 = pts[(i - 1 + n) % n], p1 = pts[i], p2 = pts[(i + 1) % n], p3 = pts[(i + 2) % n];
      d += ` C${f1(p1[0] + (p2[0] - p0[0]) / 6)} ${f1(p1[1] + (p2[1] - p0[1]) / 6)} ${f1(p2[0] - (p3[0] - p1[0]) / 6)} ${f1(p2[1] - (p3[1] - p1[1]) / 6)} ${f1(p2[0])} ${f1(p2[1])}`;
    }
    return d + ' Z';
  }
  // left half (top center -> down the left side -> bottom center) mirrored into the full outline
  const outline = left => smooth(left.concat(left.slice(1, -1).reverse().map(([x, y]) => [800 - x, y])));
  function trap(a, b, c, d, r) {
    return `M${f1(a[0])} ${f1(a[1])} L${f1(b[0])} ${f1(b[1])} L${f1(c[0] + 1)} ${f1(c[1] - r)} Q${f1(c[0])} ${f1(c[1])} ${f1(c[0] - r)} ${f1(c[1])}`
      + ` L${f1(d[0] + r)} ${f1(d[1])} Q${f1(d[0])} ${f1(d[1])} ${f1(d[0] - 1)} ${f1(d[1] - r)} Z`;
  }

  function geometry(model) {
    if (model === 'ds4') {
      // outline + part positions traced pixel-by-pixel from a real PS4-style controller photo (photo px -> view box)
      const s = 1.2, P = (x, y) => [400 + (x - 280.5) * s, 70 + (y - 93) * s];
      const left = [[280.5, 109], [194, 109], [156, 109], [122, 104.5], [92, 104], [81, 106], [58, 136], [50, 151], [41, 178], [16, 278], [9, 325],
        [7, 370], [9, 391], [13, 406], [20, 420], [34, 434], [51, 442], [72, 443], [89, 437], [101, 428], [111, 416], [123, 394], [135, 366],
        [150, 319], [156, 313], [162, 312], [174, 319], [185, 322], [201, 322], [216, 318], [232, 308], [258, 307], [280.5, 307]].map(p => P(...p));
      const tl = P(193.5, 110);
      return { w: 547 * s, body: outline(left),
        touch: trap(P(193.5, 110), P(367.5, 110), P(363.5, 202), P(197.5, 202), 12 * s), tp: [tl[0], tl[1], 174 * s, 92 * s],
        dpad: P(108, 175), dL: 43 * s, dW: 12.5 * s, face: P(453, 175), fS: 39 * s, fR: 17 * s,
        sticks: [P(189.5, 260), P(371.5, 260)], sR: 46 * s, home: P(280.5, 268), hR: 13 * s,
        share: [...P(167.5, 128.5), -12], options: [...P(393.5, 128.5), 12], pill: [14 * s, 27 * s], grille: P(280.5, 215), port: P(280.5, 300),
        bump: [P(84, 108), P(152, 100)], gripY: P(0, 352)[1], light: [P(197, 110), P(364, 110)], label: 1,
      };
    }
    if (model === 'ds5') {
      // traced from a straight-on DualSense photo (Wikimedia Commons) and mirrored for symmetry
      const s = .725, P = (x, y) => [400 + (x - 490) * s, 52 + (y - 34) * s];
      const left = [[400, 52], [357.9, 52], [275.3, 57.1], [245.6, 55.6], [199.9, 56.4], [176, 61.4], [161.5, 68], [141.9, 85.3], [117.2, 137.6],
        [102, 180.3], [84.6, 249.9], [75.9, 299.2], [70.9, 349.2], [70.1, 393.5], [76.7, 447.8], [86.1, 471.1], [109.3, 487.7], [134, 498],
        [156, 506.6], [168.7, 496.4], [178.9, 476.1], [226, 347.1], [236.9, 334], [250.7, 326.1], [271, 324.6], [318.8, 328.9], [400, 328.9]];
      const plate = [[400, 53], [292, 53], [282, 100], [276, 160], [282, 225], [300, 290], [330, 322], [400, 328]];
      const tl = P(312, 42);
      return { w: 660, body: outline(left), plate: outline(plate),
        touch: trap(P(312, 42), P(668, 42), P(638, 200), P(342, 200), 18 * s), tp: [tl[0], tl[1], 356 * s, 158 * s],
        dpad: P(213, 170), dL: 70 * s, dW: 23 * s, face: P(767, 170), fS: 63 * s, fR: 30 * s,
        sticks: [P(348, 272), P(632, 272)], sR: 68 * s, home: P(490, 282), hR: 18 * s, mute: P(490, 340), grille: P(490, 228),
        share: [...P(283, 78), -4], options: [...P(697, 78), 4], pill: [15 * s, 32 * s],
        bump: [P(165, 62), P(292, 44)], gripY: P(0, 470)[1], label: 1,
        strips: [[P(318, 52), P(340, 190)], [P(662, 52), P(640, 190)]], fn: [P(348, 352), P(632, 352)],
      };
    }
    // measured from a straight-on Xbox Series controller photo (Wikimedia Commons), averaged for symmetry
    const s = .83, P = (x, y) => [400 + (x - 512) * s, 64 + (y - 130) * s];
    const left = [[512, 130], [440, 129], [380, 127], [340, 125], [300, 131], [265, 145], [238, 165], [218, 192], [205, 225], [190, 268],
      [172, 315], [155, 365], [142, 415], [132, 465], [126, 515], [127, 560], [135, 600], [150, 630], [172, 650], [198, 657], [222, 650],
      [240, 630], [256, 598], [276, 562], [300, 532], [330, 512], [380, 503], [450, 500], [512, 500]].map(p => P(...p));
    return { w: 772 * s, body: outline(left), xbox: 1,
      dpad: P(399, 390), dL: 52 * s, dW: 18 * s, face: P(726, 266), fS: 47 * s, fR: 22 * s,
      sticks: [P(298, 264), P(625, 390)], sR: 57 * s, home: P(512, 183), hR: 29 * s,
      share: P(451, 262), options: P(573, 262), sharebtn: P(512, 303), small: 16 * s,
      bump: [P(250, 152), P(385, 126)], gripY: P(0, 560)[1],
    };
  }

  // PlayStation-style symbols (q = symbol size)
  const PSYM = {
    triangle: (x, y, q) => `<path d="M${f1(x)} ${f1(y - q)} l${f1(q)} ${f1(q * 1.7)} h${f1(-q * 2)} z"/>`,
    circle: (x, y, q) => `<circle cx="${f1(x)}" cy="${f1(y)}" r="${f1(q * .95)}"/>`,
    cross: (x, y, q) => `<path d="M${f1(x - q * .8)} ${f1(y - q * .8)} l${f1(q * 1.6)} ${f1(q * 1.6)} M${f1(x + q * .8)} ${f1(y - q * .8)} l${f1(-q * 1.6)} ${f1(q * 1.6)}"/>`,
    square: (x, y, q) => `<rect x="${f1(x - q * .85)}" y="${f1(y - q * .85)}" width="${f1(q * 1.7)}" height="${f1(q * 1.7)}" rx="1"/>`,
  };
  const PSCOL = { triangle: '#3fd3a8', circle: '#ff5a6e', cross: '#7aa8ff', square: '#ee8ccf' };
  const XBCOL = { cross: '#3db86c', circle: '#e5484d', square: '#3b82f6', triangle: '#f2c230' };
  const XBLET = { cross: 'A', circle: 'B', square: 'X', triangle: 'Y' };

  function symColor(k, src, xbox) {
    if (k.symbols === 'white') return '#f1f1f4';
    if (k.symbols === 'dark') return '#5b5f68';
    if (k.symbols === 'match') return L(k.btn, .35);
    return (xbox ? XBCOL : PSCOL)[src];
  }

  // ---------------------------------------------------------------- parts
  function faceButton(id, k, src, x, y, r, xbox) {
    const sym = k.symbols === 'none' ? '' : xbox
      ? `<text x="${f1(x)}" y="${f1(y + r * .32)}" text-anchor="middle" font-size="${f1(r * .9)}" font-weight="800" font-family="Segoe UI,Arial" fill="${symColor(k, src, true)}" class="nopt">${XBLET[src]}</text>`
      : `<g fill="none" stroke="${symColor(k, src)}" stroke-width="${f1(r * .11)}" stroke-linecap="round" stroke-linejoin="round" class="nopt">${PSYM[src](x, y, r * .43)}</g>`;
    return `<g class="k" data-src="${src}">
      <circle cx="${f1(x)}" cy="${f1(y + r * .13)}" r="${f1(r + 1)}" fill="#000" opacity=".55"/>
      <circle cx="${f1(x)}" cy="${f1(y)}" r="${f1(r)}" fill="url(#${id}-btn)" stroke="#000" stroke-opacity=".55" stroke-width="1.5"/>
      <ellipse cx="${f1(x - r * .22)}" cy="${f1(y - r * .42)}" rx="${f1(r * .5)}" ry="${f1(r * .26)}" fill="#fff" opacity="${k.finish === 'chrome' ? .28 : k.finish === 'gloss' ? .22 : .07}"/>${sym}</g>`;
  }
  const rot = ([x, y], a) => [x * Math.cos(a) - y * Math.sin(a), x * Math.sin(a) + y * Math.cos(a)];
  function psDpad(id, k, cx, cy, Ln, hw) {
    const armPts = [[-hw, -Ln], [hw, -Ln], [hw, -(hw + 3)], [0, -3], [-hw, -(hw + 3)]];
    const arrow = [[0, -Ln * .8], [hw * .45, -Ln * .8 + hw * .7], [-hw * .45, -Ln * .8 + hw * .7]];
    const poly = (pts, a) => 'M' + pts.map(p => { const [x, y] = rot(p, a); return `${f1(cx + x)} ${f1(cy + y)}`; }).join(' L') + ' Z';
    const arm = (src, a) => `<g class="k" data-src="${src}"><path d="${poly(armPts, a)}" fill="url(#${id}-dpad)" stroke="#000" stroke-opacity=".7" stroke-width="1.5" stroke-linejoin="round"/>
      ${k.symbols === 'none' ? '' : `<path d="${poly(arrow, a)}" fill="${k.finish === 'chrome' ? '#ffffff55' : L(k.dpad, .3)}" class="nopt"/>`}</g>`;
    return `<circle class="nopt" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(Ln + 12)}" fill="url(#${id}-well)" opacity=".85"/>`
      + arm('up', 0) + arm('right', Math.PI / 2) + arm('down', Math.PI) + arm('left', -Math.PI / 2);
  }
  function xbDpad(id, k, cx, cy, Ln, hw) {
    const arm = (src, x, y, w, h) => `<g class="k" data-src="${src}"><rect x="${f1(x)}" y="${f1(y)}" width="${f1(w)}" height="${f1(h)}" rx="4" fill="url(#${id}-dpad)" stroke="#000" stroke-opacity=".7" stroke-width="1.5"/></g>`;
    return `<circle class="nopt" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(Ln + 10)}" fill="url(#${id}-well)"/>
      <circle class="nopt" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(Ln + 3)}" fill="none" stroke="#000" stroke-opacity=".45" stroke-width="3"/>
      ${arm('up', cx - hw, cy - Ln, hw * 2, Ln - hw + 2)}${arm('down', cx - hw, cy + hw - 2, hw * 2, Ln - hw + 2)}
      ${arm('left', cx - Ln, cy - hw, Ln - hw + 2, hw * 2)}${arm('right', cx + hw - 2, cy - hw, Ln - hw + 2, hw * 2)}
      <rect class="nopt" x="${f1(cx - hw)}" y="${f1(cy - hw)}" width="${f1(hw * 2)}" height="${f1(hw * 2)}" fill="url(#${id}-dpad)"/><circle class="nopt" cx="${f1(cx)}" cy="${f1(cy)}" r="${f1(hw * .4)}" fill="#000" opacity=".35"/>`;
  }
  function stick(id, k, src, cx, cy, R, side, edge) {
    const ring = k.ring || (edge ? '#2c2c33' : null), c = (v) => f1(v);
    return `<g class="k stick" data-src="${src}">
      <circle cx="${c(cx)}" cy="${c(cy)}" r="${c(R)}" fill="url(#${id}-well)" stroke="${L(k.body, .08)}" stroke-opacity=".6" stroke-width="2"/>
      <circle cx="${c(cx)}" cy="${c(cy)}" r="${c(R * .81)}" fill="none" stroke="#000" stroke-opacity=".5" stroke-width="${c(R * .07)}"/>
      <g class="capg" id="${id}-${side}" style="transform-origin:${c(cx)}px ${c(cy)}px">
        <circle cx="${c(cx)}" cy="${c(cy + R * .09)}" r="${c(R * .72)}" fill="#000" opacity=".6"/>
        <circle cx="${c(cx)}" cy="${c(cy)}" r="${c(R * .72)}" fill="url(#${id}-stick)"/>
        <circle cx="${c(cx)}" cy="${c(cy)}" r="${c(R * .675)}" fill="none" stroke="#000" stroke-opacity=".55" stroke-width="${c(R * .065)}" stroke-dasharray="1.6 2.6"/>
        <circle cx="${c(cx)}" cy="${c(cy)}" r="${c(R * .54)}" fill="url(#${id}-capin)"/>
        ${ring ? `<circle cx="${c(cx)}" cy="${c(cy)}" r="${c(R * .565)}" fill="none" stroke="${ring}" stroke-width="${c(R * .055)}"/><circle cx="${c(cx)}" cy="${c(cy)}" r="${c(R * .565)}" fill="none" stroke="${L(ring, .5)}" stroke-width="1" stroke-dasharray="${c(R * .4)} ${c(R * 3)}" opacity=".7"/>` : ''}
        <ellipse cx="${c(cx - R * .15)}" cy="${c(cy - R * .26)}" rx="${c(R * .3)}" ry="${c(R * .13)}" fill="#fff" opacity=".07"/>
      </g></g>`;
  }
  function shoulders(id, k, meters, G) {
    const [[ax, ay], [bx, by]] = G.bump, dx = bx - ax;
    const X = (x, r) => f1(r ? 800 - x : x);
    const bumpD = r => `M${X(ax, r)} ${f1(ay + 2)} Q${X(ax + dx * .2, r)} ${f1(by - 10)} ${X(bx, r)} ${f1(by - 9)} L${X(bx + 4, r)} ${f1(by + 7)}`
      + ` Q${X(ax + dx * .3, r)} ${f1(by + 8)} ${X(ax + 10, r)} ${f1(ay + 14)} Z`;
    const bumpHL = r => `M${X(ax + 8, r)} ${f1(ay - 1)} Q${X(ax + dx * .25, r)} ${f1(by - 6)} ${X(bx - 6, r)} ${f1(by - 5)}`;
    const trigD = r => `M${X(ax + 10, r)} ${f1(ay - 4)} C${X(ax + 8, r)} ${f1(by - 34)} ${X(ax + dx * .45, r)} ${f1(by - 46)} ${X(bx - dx * .2, r)} ${f1(by - 45)}`
      + ` C${X(bx - 6, r)} ${f1(by - 44)} ${X(bx + 2, r)} ${f1(by - 30)} ${X(bx + 2, r)} ${f1(by - 8)} Z`;
    const trigHL = r => `M${X(ax + dx * .16, r)} ${f1(by - 18)} C${X(ax + dx * .2, r)} ${f1(by - 34)} ${X(ax + dx * .45, r)} ${f1(by - 39)} ${X(bx - dx * .25, r)} ${f1(by - 38)}`;
    const trig = (src, r) => `<g class="k trig" data-src="${src}" id="${id}-${src}">
      <path d="${trigD(r)}" fill="url(#${id}-trig)" stroke="${D(k.trig, .6)}" stroke-width="1.5"/>
      <path d="${trigHL(r)}" fill="none" stroke="#fff" stroke-opacity="${k.trigFinish === 'chrome' ? .45 : .18}" stroke-width="3" stroke-linecap="round"/>
      <path class="nopt" id="${id}-${src}p" d="${trigD(r)}" fill="var(--acc)" opacity="0"/>
      <text class="lbl" x="${X(ax + dx * .55, r)}" y="${f1(by - 22)}">${src.toUpperCase()}</text></g>`;
    const bump = (src, r) => `<g class="k" data-src="${src}"><path d="${bumpD(r)}" fill="url(#${id}-bump)" stroke="#000" stroke-width="1.5"/>
      <path d="${bumpHL(r)}" fill="none" stroke="#fff" stroke-opacity="${k.trigFinish === 'chrome' ? .5 : .15}" stroke-width="2.5" stroke-linecap="round"/></g>`;
    const mw = dx * .9, mx = ax + dx * .05;
    const m = meters ? `<g class="nopt"><rect x="${f1(mx)}" y="6" width="${f1(mw)}" height="7" rx="3.5" fill="#000" opacity=".35"/><rect id="${id}-m-l2" x="${f1(mx)}" y="6" width="0" height="7" rx="3.5" fill="var(--acc)"/>
      <rect x="${f1(800 - mx - mw)}" y="6" width="${f1(mw)}" height="7" rx="3.5" fill="#000" opacity=".35"/><rect id="${id}-m-r2" x="${f1(800 - mx - mw)}" y="6" width="0" height="7" rx="3.5" fill="var(--acc)" data-right="1"/></g>` : '';
    return m + trig('l2', 0) + trig('r2', 1) + bump('l1', 0) + bump('r1', 1);
  }

  // ---------------------------------------------------------------- build
  function svg(id, vizIn, opts) {
    opts = opts || {};
    const { viz, skin: k } = resolve(vizIn);
    const G = geometry(viz.model), xbox = viz.model === 'xbox', ds5 = viz.model === 'ds5', ds4 = viz.model === 'ds4';
    const edge = ds5 && viz.version === 'edge';
    const u = n => `url(#${id}-${n})`;
    const glow = opts.glow !== false;
    const bodyFill = k.texture === 'camo' ? u('camo') : u('body');
    const gw = G.w * .3;
    let html = `<svg class="pad ehp${glow ? '' : ' noglow'}" id="${id}" viewBox="0 0 800 520" style="--acc:${k.accent};--light:${k.accent}"
      data-tp="${G.tp ? G.tp.map(f1).join(',') : ''}" data-tilt="${f1(G.sR * .28)}" data-mw="${f1((G.bump[1][0] - G.bump[0][0]) * .9)}">
    <defs>
      <linearGradient id="${id}-body" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${L(k.body, .1)}"/><stop offset=".42" stop-color="${k.body}"/><stop offset="1" stop-color="${D(k.body, .45)}"/></linearGradient>
      <linearGradient id="${id}-plate" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${L(k.center || k.body, .08)}"/><stop offset="1" stop-color="${D(k.center || k.body, .4)}"/></linearGradient>
      <radialGradient id="${id}-sheen" cx=".5" cy=".1" r=".8"><stop offset="0" stop-color="#fff" stop-opacity="${lum(k.body) > .5 ? .35 : .14}"/><stop offset=".6" stop-color="#fff" stop-opacity=".02"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient>
      <radialGradient id="${id}-well" cx=".5" cy=".42" r=".6"><stop offset="0" stop-color="#040405"/><stop offset=".82" stop-color="${D(k.body, .7)}"/><stop offset="1" stop-color="${D(k.body, .25)}"/></radialGradient>
      <radialGradient id="${id}-capin" cx=".5" cy=".68" r=".65"><stop offset="0" stop-color="${L(k.stick, .14)}"/><stop offset="1" stop-color="${D(k.stick, .45)}"/></radialGradient>
      ${fill(id, 'btn', k.btn, k.finish)}${fill(id, 'dpad', k.dpad, k.finish)}${fill(id, 'trig', k.trig, k.trigFinish)}${fill(id, 'bump', k.trig, k.trigFinish)}
      ${fill(id, 'stick', k.stick, 'matte')}${fill(id, 'tpad', k.touch, k.touchFinish)}${fill(id, 'home', k.home, 'gloss')}
      ${textures(id, k)}
      <filter id="${id}-sh" x="-15%" y="-15%" width="130%" height="140%"><feDropShadow dx="0" dy="14" stdDeviation="14" flood-color="#000" flood-opacity=".7"/></filter>
      <filter id="${id}-glow" x="-50%" y="-200%" width="200%" height="500%"><feGaussianBlur stdDeviation="6"/></filter>
      <clipPath id="${id}-clip"><path d="${G.body}"/></clipPath>
      ${G.touch ? `<clipPath id="${id}-tpclip"><path d="${G.touch}"/></clipPath>` : ''}
    </defs>`;
    html += shoulders(id, k, opts.meters, G);
    if (ds4 && viz.version === 'v1') html += `<path class="nopt" d="M${f1(G.light[0][0])} ${f1(G.light[0][1] - 4)} L${f1(G.light[1][0])} ${f1(G.light[1][1] - 4)}" stroke-width="10" stroke-linecap="round" filter="${u('glow')}" style="stroke:var(--light)" opacity=".8"/>`;
    // shell
    html += `<path d="${G.body}" fill="${bodyFill}" ${opts.shadow === false ? '' : `filter="${u('sh')}"`}/>
      ${k.texture === 'carbon' ? `<path class="nopt" d="${G.body}" fill="${u('carbon')}" opacity=".9"/><path class="nopt" d="${G.body}" fill="${u('body')}" opacity=".35"/>` : ''}
      ${k.texture === 'grain' ? `<path class="nopt" d="${G.body}" fill="${u('grain')}"/>` : ''}
      ${G.plate && k.center ? `<path class="nopt" d="${G.plate}" fill="${u('plate')}"/>` : ''}
      ${k.gripTex ? `<g clip-path="${u('clip')}" class="nopt"><path d="M0 ${f1(G.gripY)} H${f1(400 - gw)} V520 H0 Z M800 ${f1(G.gripY)} H${f1(400 + gw)} V520 H800 Z" fill="${u('gripdots')}"/>
          <path d="M0 ${f1(G.gripY)} H${f1(400 - gw)} M800 ${f1(G.gripY)} H${f1(400 + gw)}" stroke="#000" stroke-opacity=".35" stroke-width="2"/></g>` : ''}
      <path class="nopt" d="${G.body}" fill="${u('sheen')}"/>
      <path class="nopt" d="${G.body}" fill="none" stroke="${lum(k.body) > .5 ? '#00000033' : '#ffffff1f'}" stroke-width="1.5"/>`;
    // light + touchpad
    if (G.touch) {
      if (ds4 && viz.version === 'v2') {
        const [a, b] = G.light;
        html += `<path class="nopt" d="M${f1(a[0])} ${f1(a[1])} L${f1(b[0])} ${f1(b[1])}" stroke-width="12" stroke-linecap="round" filter="${u('glow')}" opacity=".9" style="stroke:var(--light)"/><path class="nopt" d="M${f1(a[0])} ${f1(a[1])} L${f1(b[0])} ${f1(b[1])}" stroke-width="5" stroke-linecap="round" style="stroke:var(--light)"/>`;
      }
      html += `<g class="k" data-src="touch"><path d="${G.touch}" fill="${u('tpad')}" stroke="#000" stroke-width="2"/>
        ${k.touchFinish === 'brushed' ? `<path d="${G.touch}" fill="${u('hair')}" class="nopt"/>` : ''}
        <path d="${G.touch}" fill="none" stroke="#fff" stroke-opacity="${k.touchFinish === 'brushed' ? .25 : .06}" stroke-width="1.5" class="nopt" transform="translate(0,1.5)"/>
        <g clip-path="${u('tpclip')}" id="${id}-touch"></g></g>`;
      if (G.strips) G.strips.forEach(([a, b]) => {
        html += `<path class="nopt" d="M${f1(a[0])} ${f1(a[1])} L${f1(b[0])} ${f1(b[1])}" stroke-width="9" stroke-linecap="round" filter="${u('glow')}" style="stroke:var(--light)" opacity=".9"/>
          <path class="nopt" d="M${f1(a[0])} ${f1(a[1])} L${f1(b[0])} ${f1(b[1])}" stroke-width="3" stroke-linecap="round" style="stroke:var(--light)"/>`;
      });
    }
    // d-pad + face buttons
    html += xbox ? xbDpad(id, k, ...G.dpad, G.dL, G.dW) : psDpad(id, k, ...G.dpad, G.dL, G.dW);
    const [fx, fy] = G.face, sp = G.fS, r = G.fR;
    html += `<circle class="nopt" cx="${f1(fx)}" cy="${f1(fy)}" r="${f1(sp + r + 8)}" fill="${u('well')}" opacity="${xbox ? .45 : .85}"/>`
      + faceButton(id, k, 'triangle', fx, fy - sp, r, xbox) + faceButton(id, k, 'circle', fx + sp, fy, r, xbox)
      + faceButton(id, k, 'cross', fx, fy + sp, r, xbox) + faceButton(id, k, 'square', fx - sp, fy, r, xbox);
    // small buttons
    if (xbox) {
      const [hx2, hy] = G.home, hr = G.hR, sm = G.small;
      html += `<g class="k" data-src="ps"><circle cx="${f1(hx2)}" cy="${f1(hy)}" r="${f1(hr)}" fill="${u('home')}" stroke="#000" stroke-width="1.5"/>
          <circle cx="${f1(hx2)}" cy="${f1(hy)}" r="${f1(hr)}" fill="none" stroke-width="2.5" style="stroke:var(--light)" opacity=".9"/>
          <circle cx="${f1(hx2)}" cy="${f1(hy)}" r="${f1(hr)}" fill="none" stroke-width="8" filter="${u('glow')}" style="stroke:var(--light)" opacity=".5" class="nopt"/>
          <circle cx="${f1(hx2)}" cy="${f1(hy)}" r="${f1(hr * .4)}" fill="none" stroke="#ffffff55" stroke-width="2" class="nopt"/></g>`;
      const [sx, sy] = G.share, [ox, oy] = G.options;
      html += `<g class="k" data-src="share"><circle cx="${f1(sx)}" cy="${f1(sy)}" r="${f1(sm)}" fill="${u('btn')}" stroke="#000" stroke-width="1.5"/><path d="M${f1(sx - 5)} ${f1(sy - 2)} h6 v5 h-6z M${f1(sx - 2)} ${f1(sy - 5)} h6 v5" fill="none" stroke="#ffffff66" stroke-width="1.2" class="nopt"/></g>
        <g class="k" data-src="options"><circle cx="${f1(ox)}" cy="${f1(oy)}" r="${f1(sm)}" fill="${u('btn')}" stroke="#000" stroke-width="1.5"/><path d="M${f1(ox - 5)} ${f1(oy - 4)} h10 M${f1(ox - 5)} ${f1(oy)} h10 M${f1(ox - 5)} ${f1(oy + 4)} h10" stroke="#ffffff66" stroke-width="1.4" class="nopt"/></g>`;
      if (viz.version === 'series') { const [qx, qy] = G.sharebtn; html += `<g class="k" data-src="touch"><rect x="${f1(qx - sm * .9)}" y="${f1(qy - sm * .55)}" width="${f1(sm * 1.8)}" height="${f1(sm * 1.1)}" rx="${f1(sm * .55)}" fill="${u('btn')}" stroke="#000" stroke-width="1.5"/></g>`; }
    } else {
      const [pw, ph] = G.pill;
      const pill = (src, x, y, rt) => `<g class="k" data-src="${src}"><rect x="${f1(x - pw / 2)}" y="${f1(y - ph / 2)}" width="${f1(pw)}" height="${f1(ph)}" rx="${f1(pw / 2)}" fill="${u('btn')}" stroke="#000" stroke-width="1.5" transform="rotate(${rt} ${f1(x)} ${f1(y)})"/></g>`;
      html += pill('share', ...G.share) + pill('options', ...G.options);
      if (G.grille) html += `<g class="nopt" fill="#000" opacity=".8">${[...Array(15)].map((_, i) => `<circle cx="${f1(G.grille[0] - 14 + (i % 5) * 7)}" cy="${f1(G.grille[1] - 6 + Math.floor(i / 5) * 6)}" r="1.6"/>`).join('')}</g>`;
      const [hx2, hy] = G.home, hr = G.hR;
      html += `<g class="k" data-src="ps"><circle cx="${f1(hx2)}" cy="${f1(hy)}" r="${f1(hr)}" fill="${u('home')}" stroke="#000" stroke-width="1.5"/>
          <circle cx="${f1(hx2)}" cy="${f1(hy)}" r="${f1(hr)}" fill="none" stroke="var(--acc)" stroke-opacity=".55" stroke-width="1.2"/>
          <path class="nopt" d="M${f1(hx2 - hr * .4)} ${f1(hy + hr * .05)} l${f1(hr * .4)} ${f1(-hr * .4)} l${f1(hr * .4)} ${f1(hr * .4)} M${f1(hx2 - hr * .27)} ${f1(hy - hr * .07)} v${f1(hr * .4)} h${f1(hr * .54)} v${f1(-hr * .4)}" fill="none" stroke="#ffffff66" stroke-width="1.3" stroke-linejoin="round"/></g>`;
      if (G.mute) html += `<rect class="nopt" x="${f1(G.mute[0] - 13)}" y="${f1(G.mute[1] - 4)}" width="26" height="8" rx="4" fill="${u('btn')}" stroke="#000"/>`;
      if (edge) G.fn.forEach(([x, y]) => { html += `<rect class="nopt" x="${f1(x - 16)}" y="${f1(y - 4)}" width="32" height="8" rx="4" fill="${u('btn')}" stroke="#000"/>`; });
      if (G.port) html += `<rect class="nopt" x="${f1(G.port[0] - 22)}" y="${f1(G.port[1] - 5)}" width="44" height="9" rx="4" fill="#050506" stroke="${L(k.body, .1)}" stroke-opacity=".4"/>`;
    }
    // sticks
    html += stick(id, k, 'l3', ...G.sticks[0], G.sR, 'ls', edge) + stick(id, k, 'r3', ...G.sticks[1], G.sR, 'rs', edge);
    return html + '</svg>';
  }

  // ---------------------------------------------------------------- live update
  function update(id, S, light) {
    const el = document.getElementById(id); if (!el) return;
    const b = (S && S.btn) || {};
    el.querySelectorAll('.k').forEach(k => k.classList.toggle('on', !!b[k.dataset.src]));
    if (light) el.style.setProperty('--light', light);
    if (!S || !S.connected) return;
    const tilt = (side, x, y, pressed) => { const c = el.querySelector('#' + id + '-' + side); if (!c) return;
      const tl = +el.dataset.tilt || 15; c.style.transform = `translate(${(x - 127.5) / 127.5 * tl}px,${(y - 127.5) / 127.5 * tl}px) scale(${pressed ? .93 : 1})`; };
    tilt('ls', S.lx, S.ly, b.l3); tilt('rs', S.rx, S.ry, b.r3);
    for (const t of ['l2', 'r2']) {
      const v = (t === 'l2' ? S.l2v : S.r2v) / 255;
      const g = el.querySelector('#' + id + '-' + t); if (g) g.style.transform = `translateY(${v * 12}px) scaleY(${1 - v * .12})`;
      const p = el.querySelector('#' + id + '-' + t + 'p'); if (p) p.setAttribute('opacity', v * .5);
      const m = el.querySelector('#' + id + '-m-' + t); if (m) { const mw = +el.dataset.mw || 128; m.setAttribute('width', mw * v); if (m.dataset.right) m.setAttribute('x', +m.previousElementSibling.getAttribute('x') + mw * (1 - v)); }
    }
    const tg = el.querySelector('#' + id + '-touch');
    if (tg) {
      const [x0, y0, w, h] = (el.dataset.tp || '0,0,0,0').split(',').map(Number);
      const key = JSON.stringify(S.touches || []);
      if (tg._k !== key) { tg._k = key;
        tg.innerHTML = (S.touches || []).map(t => { const x = x0 + t[0] / 1920 * w, y = y0 + t[1] / 942 * h;
          return `<circle cx="${x}" cy="${y}" r="15" fill="var(--acc)" opacity=".4"/><circle cx="${x}" cy="${y}" r="5" fill="#fff"/>`; }).join(''); }
    }
  }

  // swatch preview for skin cards
  function swatch(skinId) {
    const { skin: k } = resolve({ skin: skinId });
    return [k.body, k.center || k.body, k.btn, k.touch, k.accent];
  }

  // ---------------------------------------------------------------- styles (injected once)
  const css = `.ehp{overflow:visible}
    .ehp .k{cursor:pointer;transition:transform .045s ease-out,filter .045s;transform-box:fill-box;transform-origin:center}
    .ehp .k.on:not(.trig):not(.stick){transform:scale(.9) translateY(1.5px);filter:brightness(1.5) drop-shadow(0 0 6px var(--acc)) drop-shadow(0 0 14px var(--acc))}
    .ehp.noglow .k.on:not(.trig):not(.stick){filter:brightness(1.45)}
    .ehp .trig{transition:transform .03s linear}
    .ehp .stick .capg{transition:transform .02s linear}
    .ehp .stick.on .capg{filter:brightness(1.35) drop-shadow(0 0 8px var(--acc))}
    .ehp .lbl{fill:#fff;fill-opacity:.38;font:600 10px Segoe UI,Arial;letter-spacing:.8px;pointer-events:none;text-anchor:middle}
    .ehp .nopt{pointer-events:none}`;
  if (typeof document !== 'undefined' && !document.getElementById('ehp-css')) {
    const st = document.createElement('style'); st.id = 'ehp-css'; st.textContent = css; document.head.appendChild(st);
  }

  window.EHPad = { MODELS, SKINS, DEF, resolve, svg, update, swatch };
})();
