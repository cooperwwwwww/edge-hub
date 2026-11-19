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

  // ---------------------------------------------------------------- shapes per model
  const SHAPES = {
    ds4: {
      body: 'M215 112 C262 98 300 96 330 98 L470 98 C500 96 538 98 585 112 C650 118 690 150 712 210 C735 280 760 370 762 420 C764 470 735 500 700 494 C670 490 650 470 630 440 L588 392 C570 378 545 372 520 372 L280 372 C255 372 230 378 212 392 L170 440 C150 470 130 490 100 494 C65 500 36 470 38 420 C40 370 65 280 88 210 C110 150 150 118 215 112 Z',
      grips: 'M30 398 C90 372 160 380 222 404 L180 440 C150 470 130 490 100 494 C65 500 36 470 38 420 Z M770 398 C710 372 640 380 578 404 L620 440 C650 470 670 490 700 494 C735 500 764 470 762 420 Z',
      touch: 'M304 102 L496 102 L492 192 Q490 208 474 208 L326 208 Q310 208 308 192 Z', tp: [304, 102, 192, 106],
      dpad: [190, 210], face: [610, 210], sticks: [[300, 300], [500, 300]], home: [400, 300],
      share: [274, 139, -18], options: [526, 139, 18], grille: [400, 245],
    },
    ds5: {
      body: 'M200 118 C250 100 300 98 330 100 L470 100 C500 98 550 100 600 118 C665 128 700 165 718 225 C740 300 768 385 768 430 C768 478 738 506 702 500 C672 495 652 476 632 446 L592 398 C572 382 548 376 520 376 L280 376 C252 376 228 382 208 398 L168 446 C148 476 128 495 98 500 C62 506 32 478 32 430 C32 385 60 300 82 225 C100 165 135 128 200 118 Z',
      plate: 'M292 104 L508 104 C528 150 548 210 566 270 C578 320 562 370 520 376 L280 376 C238 370 222 320 234 270 C252 210 272 150 292 104 Z',
      grips: 'M26 400 C88 374 160 382 224 406 L168 446 C148 476 128 495 98 500 C62 506 32 478 32 430 Z M774 400 C712 374 640 382 576 406 L632 446 C652 476 672 495 702 500 C738 506 768 478 768 430 Z',
      touch: 'M298 104 L502 104 L497 214 Q495 230 478 230 L322 230 Q305 230 303 214 Z', tp: [298, 104, 204, 126],
      dpad: [186, 214], face: [614, 214], sticks: [[300, 310], [500, 310]], home: [400, 268],
      share: [278, 130, -22], options: [522, 130, 22],
    },
    xbox: {
      body: 'M220 120 C270 104 330 108 400 112 C470 108 530 104 580 120 C640 130 680 160 705 220 C730 285 760 380 760 430 C760 478 728 502 694 496 C664 490 640 468 620 440 L585 392 C565 372 540 362 510 360 L290 360 C260 362 235 372 215 392 L180 440 C160 468 136 490 106 496 C72 502 40 478 40 430 C40 380 70 285 95 220 C120 160 160 130 220 120 Z',
      grips: 'M34 402 C94 376 162 384 222 404 L180 440 C160 468 136 490 106 496 C72 502 40 478 40 430 Z M766 402 C706 376 638 384 578 404 L620 440 C640 468 664 490 694 496 C728 502 760 478 760 430 Z',
      dpad: [305, 296], face: [592, 205], sticks: [[212, 205], [495, 296]], home: [400, 150],
      share: [352, 205], options: [448, 205],
    },
  };

  // PlayStation-style symbols
  const PSYM = {
    triangle: (x, y) => `<path d="M${x} ${y - 10} l10 17 h-20 z"/>`,
    circle: (x, y) => `<circle cx="${x}" cy="${y}" r="9.5"/>`,
    cross: (x, y) => `<path d="M${x - 8} ${y - 8} l16 16 M${x + 8} ${y - 8} l-16 16"/>`,
    square: (x, y) => `<rect x="${x - 8.5}" y="${y - 8.5}" width="17" height="17" rx="1"/>`,
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
      ? `<text x="${x}" y="${y + 7}" text-anchor="middle" font-size="20" font-weight="800" font-family="Segoe UI,Arial" fill="${symColor(k, src, true)}" class="nopt">${XBLET[src]}</text>`
      : `<g fill="none" stroke="${symColor(k, src)}" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" class="nopt">${PSYM[src](x, y)}</g>`;
    return `<g class="k" data-src="${src}">
      <circle cx="${x}" cy="${y + 3}" r="${r + 1}" fill="#000" opacity=".55"/>
      <circle cx="${x}" cy="${y}" r="${r}" fill="url(#${id}-btn)" stroke="#000" stroke-opacity=".55" stroke-width="1.5"/>
      <ellipse cx="${x - r * .22}" cy="${y - r * .42}" rx="${r * .5}" ry="${r * .26}" fill="#fff" opacity="${k.finish === 'chrome' ? .28 : k.finish === 'gloss' ? .22 : .07}"/>${sym}</g>`;
  }
  function psDpad(id, k, cx, cy) {
    const arm = (src, d, tri) => `<g class="k" data-src="${src}"><path d="${d}" fill="url(#${id}-dpad)" stroke="#000" stroke-opacity=".7" stroke-width="1.5"/>
      ${k.symbols === 'none' ? '' : `<path d="${tri}" fill="${k.finish === 'chrome' ? '#ffffff55' : L(k.dpad, .3)}" class="nopt"/>`}</g>`;
    return `<circle class="nopt" cx="${cx}" cy="${cy}" r="66" fill="url(#${id}-well)" opacity=".85"/>
      ${arm('up', `M${cx - 16} ${cy - 56} h32 v36 l-16 16 l-16 -16 z`, `M${cx} ${cy - 46} l7 10 h-14 z`)}
      ${arm('down', `M${cx - 16} ${cy + 56} h32 v-36 l-16 -16 l-16 16 z`, `M${cx} ${cy + 46} l7 -10 h-14 z`)}
      ${arm('left', `M${cx - 56} ${cy - 16} v32 h36 l16 -16 l-16 -16 z`, `M${cx - 46} ${cy} l10 -7 v14 z`)}
      ${arm('right', `M${cx + 56} ${cy - 16} v32 h-36 l-16 -16 l16 -16 z`, `M${cx + 46} ${cy} l-10 -7 v14 z`)}`;
  }
  function xbDpad(id, k, cx, cy) {
    const arm = (src, x, y, w, h) => `<g class="k" data-src="${src}"><rect x="${x}" y="${y}" width="${w}" height="${h}" rx="5" fill="url(#${id}-dpad)" stroke="#000" stroke-opacity=".7" stroke-width="1.5"/></g>`;
    return `<circle class="nopt" cx="${cx}" cy="${cy}" r="52" fill="url(#${id}-well)"/>
      <circle class="nopt" cx="${cx}" cy="${cy}" r="44" fill="none" stroke="#000" stroke-opacity=".45" stroke-width="3"/>
      ${arm('up', cx - 14, cy - 44, 28, 34)}${arm('down', cx - 14, cy + 10, 28, 34)}${arm('left', cx - 44, cy - 14, 34, 28)}${arm('right', cx + 10, cy - 14, 34, 28)}
      <rect class="nopt" x="${cx - 14}" y="${cy - 14}" width="28" height="28" fill="url(#${id}-dpad)"/><circle class="nopt" cx="${cx}" cy="${cy}" r="6" fill="#000" opacity=".35"/>`;
  }
  function stick(id, k, src, cx, cy, side, edge) {
    const ring = k.ring || (edge ? '#2c2c33' : null);
    return `<g class="k stick" data-src="${src}">
      <circle cx="${cx}" cy="${cy}" r="54" fill="url(#${id}-well)" stroke="${L(k.body, .08)}" stroke-opacity=".6" stroke-width="2"/>
      <circle cx="${cx}" cy="${cy}" r="44" fill="none" stroke="#000" stroke-opacity=".5" stroke-width="4"/>
      <g class="capg" id="${id}-${side}" style="transform-origin:${cx}px ${cy}px">
        <circle cx="${cx}" cy="${cy + 5}" r="39" fill="#000" opacity=".6"/>
        <circle cx="${cx}" cy="${cy}" r="39" fill="url(#${id}-stick)"/>
        <circle cx="${cx}" cy="${cy}" r="36.5" fill="none" stroke="#000" stroke-opacity=".55" stroke-width="3.5" stroke-dasharray="1.6 2.6"/>
        <circle cx="${cx}" cy="${cy}" r="29" fill="url(#${id}-capin)"/>
        ${ring ? `<circle cx="${cx}" cy="${cy}" r="30.5" fill="none" stroke="${ring}" stroke-width="3"/><circle cx="${cx}" cy="${cy}" r="30.5" fill="none" stroke="${L(ring, .5)}" stroke-width="1" stroke-dasharray="20 160" opacity=".7"/>` : ''}
        <circle class="capring" cx="${cx}" cy="${cy}" r="29" fill="none" stroke="#fff" stroke-opacity=".06" stroke-width="2"/>
        <ellipse cx="${cx - 8}" cy="${cy - 14}" rx="16" ry="7" fill="#fff" opacity=".07"/>
      </g></g>`;
  }
  function shoulders(id, k, meters) {
    const trig = (src, d, hl, lx) => `<g class="k trig" data-src="${src}" id="${id}-${src}">
      <path d="${d}" fill="url(#${id}-trig)" stroke="${D(k.trig, .6)}" stroke-width="1.5"/>
      <path d="${hl}" fill="none" stroke="#fff" stroke-opacity="${k.trigFinish === 'chrome' ? .45 : .18}" stroke-width="3" stroke-linecap="round"/>
      <path class="nopt" id="${id}-${src}p" d="${d}" fill="var(--acc)" opacity="0"/>
      <text class="lbl" x="${lx}" y="92">${src.toUpperCase()}</text></g>`;
    const bump = (src, d, edge, lx) => `<g transform="translate(0,-17)"><g class="k" data-src="${src}">
      <path d="${d}" fill="url(#${id}-bump)" stroke="#000" stroke-width="1.5"/>
      <path d="${edge}" fill="none" stroke="#fff" stroke-opacity="${k.trigFinish === 'chrome' ? .5 : .15}" stroke-width="2.5" stroke-linecap="round"/>
      <text class="lbl" x="${lx}" y="124" font-size="9">${src.toUpperCase()}</text></g></g>`;
    const m = meters ? `<g class="nopt"><rect x="172" y="24" width="128" height="7" rx="3.5" fill="#000" opacity=".35"/><rect id="${id}-m-l2" x="172" y="24" width="0" height="7" rx="3.5" fill="var(--acc)"/>
      <rect x="500" y="24" width="128" height="7" rx="3.5" fill="#000" opacity=".35"/><rect id="${id}-m-r2" x="500" y="24" width="0" height="7" rx="3.5" fill="var(--acc)"/></g>` : '';
    return m + trig('l2', 'M172 124 C170 80 205 52 252 50 C284 49 300 70 300 100 L298 122 Z', 'M190 104 C196 78 222 64 252 63', 245)
      + trig('r2', 'M628 124 C630 80 595 52 548 50 C516 49 500 70 500 100 L502 122 Z', 'M610 104 C604 78 578 64 548 63', 555)
      + bump('l1', 'M150 150 C168 118 222 100 300 101 L301 122 C240 120 192 130 166 160 Z', 'M160 146 C182 122 226 110 296 108', 232)
      + bump('r1', 'M650 150 C632 118 578 100 500 101 L499 122 C560 120 608 130 634 160 Z', 'M640 146 C618 122 574 110 504 108', 568);
  }

  // ---------------------------------------------------------------- build
  function svg(id, vizIn, opts) {
    opts = opts || {};
    const { viz, skin: k } = resolve(vizIn);
    const S = SHAPES[viz.model], xbox = viz.model === 'xbox', ds5 = viz.model === 'ds5', ds4 = viz.model === 'ds4';
    const edge = ds5 && viz.version === 'edge';
    const u = n => `url(#${id}-${n})`;
    const glow = opts.glow !== false;
    const bodyFill = k.texture === 'camo' ? u('camo') : u('body');
    const btnStroke = '#000';
    let html = `<svg class="pad ehp${glow ? '' : ' noglow'}" id="${id}" viewBox="0 0 800 520" style="--acc:${k.accent};--light:${k.accent}" data-tp="${S.tp ? S.tp.join(',') : ''}">
    <defs>
      <linearGradient id="${id}-body" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${L(k.body, .1)}"/><stop offset=".42" stop-color="${k.body}"/><stop offset="1" stop-color="${D(k.body, .45)}"/></linearGradient>
      <linearGradient id="${id}-plate" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${L(k.center || k.body, .08)}"/><stop offset="1" stop-color="${D(k.center || k.body, .4)}"/></linearGradient>
      <radialGradient id="${id}-sheen" cx=".5" cy=".1" r=".8"><stop offset="0" stop-color="#fff" stop-opacity="${lum(k.body) > .5 ? .35 : .14}"/><stop offset=".6" stop-color="#fff" stop-opacity=".02"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient>
      <radialGradient id="${id}-well" cx=".5" cy=".42" r=".6"><stop offset="0" stop-color="#040405"/><stop offset=".82" stop-color="${D(k.body, .7)}"/><stop offset="1" stop-color="${D(k.body, .25)}"/></radialGradient>
      <radialGradient id="${id}-capin" cx=".5" cy=".68" r=".65"><stop offset="0" stop-color="${L(k.stick, .14)}"/><stop offset="1" stop-color="${D(k.stick, .45)}"/></radialGradient>
      ${fill(id, 'btn', k.btn, k.finish)}${fill(id, 'dpad', k.dpad, k.finish)}${fill(id, 'trig', k.trig, k.trigFinish)}${fill(id, 'bump', k.trig, k.trigFinish)}
      ${fill(id, 'stick', k.stick, 'matte')}${fill(id, 'tpad', k.touch, k.touchFinish)}${fill(id, 'home', k.home, 'gloss')}
      ${textures(id, k)}
      <filter id="${id}-sh" x="-15%" y="-15%" width="130%" height="140%"><feDropShadow dx="0" dy="14" stdDeviation="14" flood-color="#000" flood-opacity="${opts.shadow === false ? 0 : .7}"/></filter>
      <filter id="${id}-glow" x="-50%" y="-200%" width="200%" height="500%"><feGaussianBlur stdDeviation="6"/></filter>
      <clipPath id="${id}-clip"><path d="${S.body}"/></clipPath>
      ${S.touch ? `<clipPath id="${id}-tpclip"><path d="${S.touch}"/></clipPath>` : ''}
    </defs>`;
    html += shoulders(id, k, opts.meters);
    if (ds4 && viz.version === 'v1') html += `<path class="nopt" d="M318 99 L482 99" stroke-width="10" stroke-linecap="round" filter="${u('glow')}" style="stroke:var(--light)" opacity=".8"/>`;
    // shell
    html += `<path d="${S.body}" fill="${bodyFill}" ${opts.shadow === false ? '' : `filter="${u('sh')}"`}/>
      ${k.texture === 'carbon' ? `<path class="nopt" d="${S.body}" fill="${u('carbon')}" opacity=".9"/><path class="nopt" d="${S.body}" fill="${u('body')}" opacity=".35"/>` : ''}
      ${k.texture === 'grain' ? `<path class="nopt" d="${S.body}" fill="${u('grain')}"/>` : ''}
      ${S.plate && k.center ? `<path class="nopt" d="${S.plate}" fill="${u('plate')}"/>` : ''}
      ${k.gripTex ? `<g clip-path="${u('clip')}" class="nopt"><path d="${S.grips}" fill="${u('gripdots')}"/><path d="${S.grips}" fill="none" stroke="#000" stroke-opacity=".5" stroke-width="2"/></g>` : ''}
      <path class="nopt" d="${S.body}" fill="${u('sheen')}"/>
      <path class="nopt" d="${S.body}" fill="none" stroke="${lum(k.body) > .5 ? '#00000033' : '#ffffff1f'}" stroke-width="1.5"/>`;
    // light + touchpad
    if (S.touch) {
      if (ds4 && viz.version === 'v2') html += `<path class="nopt" d="M316 96 L484 96" stroke-width="12" stroke-linecap="round" filter="${u('glow')}" opacity=".9" style="stroke:var(--light)"/><path class="nopt" d="M316 96 L484 96" stroke-width="5" stroke-linecap="round" style="stroke:var(--light)"/>`;
      html += `<g class="k" data-src="touch"><path d="${S.touch}" fill="${u('tpad')}" stroke="#000" stroke-width="2"/>
        ${k.touchFinish === 'brushed' ? `<path d="${S.touch}" fill="${u('hair')}" class="nopt"/>` : ''}
        <path d="${S.touch}" fill="none" stroke="#fff" stroke-opacity="${k.touchFinish === 'brushed' ? .25 : .06}" stroke-width="1.5" class="nopt" transform="translate(0,1.5)"/>
        <g clip-path="${u('tpclip')}" id="${id}-touch"></g></g>`;
      if (ds5) {
        const [x, y, w, h] = S.tp;
        html += `<g class="nopt"><path d="M${x + 2} ${y + 10} L${x + 6} ${y + h - 10}" stroke-width="10" stroke-linecap="round" filter="${u('glow')}" style="stroke:var(--light)" opacity=".9"/>
          <path d="M${x + w - 2} ${y + 10} L${x + w - 6} ${y + h - 10}" stroke-width="10" stroke-linecap="round" filter="${u('glow')}" style="stroke:var(--light)" opacity=".9"/>
          <path d="M${x + 2} ${y + 10} L${x + 6} ${y + h - 10} M${x + w - 2} ${y + 10} L${x + w - 6} ${y + h - 10}" stroke-width="3.5" stroke-linecap="round" style="stroke:var(--light)"/></g>`;
      }
    }
    // d-pad + face buttons
    html += xbox ? xbDpad(id, k, ...S.dpad) : psDpad(id, k, ...S.dpad);
    const [fx, fy] = S.face, sp = xbox ? 40 : 46;
    html += `<circle class="nopt" cx="${fx}" cy="${fy}" r="${xbox ? 66 : 74}" fill="${u('well')}" opacity="${xbox ? .5 : .85}"/>`
      + faceButton(id, k, 'triangle', fx, fy - sp, xbox ? 21 : 23, xbox) + faceButton(id, k, 'circle', fx + sp, fy, xbox ? 21 : 23, xbox)
      + faceButton(id, k, 'cross', fx, fy + sp, xbox ? 21 : 23, xbox) + faceButton(id, k, 'square', fx - sp, fy, xbox ? 21 : 23, xbox);
    // small buttons
    const pill = (src, x, y, rot, w = 13, h = 30) => `<g class="k" data-src="${src}"><rect x="${x - w / 2}" y="${y - h / 2}" width="${w}" height="${h}" rx="${w / 2}" fill="${u('btn')}" stroke="${btnStroke}" stroke-width="1.5" transform="rotate(${rot} ${x} ${y})"/></g>`;
    if (xbox) {
      const [hx2, hy] = S.home;
      html += `<g class="k" data-src="ps"><circle cx="${hx2}" cy="${hy}" r="23" fill="${u('home')}" stroke="#000" stroke-width="1.5"/>
          <circle cx="${hx2}" cy="${hy}" r="23" fill="none" stroke-width="2.5" style="stroke:var(--light)" opacity=".9"/>
          <circle cx="${hx2}" cy="${hy}" r="23" fill="none" stroke-width="8" filter="${u('glow')}" style="stroke:var(--light)" opacity=".5" class="nopt"/>
          <circle cx="${hx2}" cy="${hy}" r="9" fill="none" stroke="#ffffff55" stroke-width="2" class="nopt"/></g>
        <g class="k" data-src="share"><circle cx="${S.share[0]}" cy="${S.share[1]}" r="12" fill="${u('btn')}" stroke="#000" stroke-width="1.5"/><path d="M${S.share[0] - 5} ${S.share[1] - 3} h7 v6 h-7z M${S.share[0] - 2} ${S.share[1] - 5} h7 v6" fill="none" stroke="#ffffff66" stroke-width="1.2" class="nopt"/></g>
        <g class="k" data-src="options"><circle cx="${S.options[0]}" cy="${S.options[1]}" r="12" fill="${u('btn')}" stroke="#000" stroke-width="1.5"/><path d="M${S.options[0] - 5} ${S.options[1] - 4} h10 M${S.options[0] - 5} ${S.options[1]} h10 M${S.options[0] - 5} ${S.options[1] + 4} h10" stroke="#ffffff66" stroke-width="1.4" class="nopt"/></g>
        ${viz.version === 'series' ? pill('touch', 400, 240, 90, 11, 22) : ''}`;
    } else {
      html += pill('share', ...S.share) + pill('options', ...S.options)
        + `<text class="lbl" x="${S.share[0] - 12}" y="${S.share[1] + 33}" font-size="8">${ds5 ? 'CREATE' : 'SHARE'}</text><text class="lbl" x="${S.options[0] + 14}" y="${S.options[1] + 33}" font-size="8">OPTIONS</text>`;
      if (S.grille) html += `<g class="nopt" fill="#000" opacity=".8">${[...Array(15)].map((_, i) => `<circle cx="${S.grille[0] - 16 + (i % 5) * 8}" cy="${S.grille[1] - 7 + Math.floor(i / 5) * 7}" r="1.8"/>`).join('')}</g>`;
      const [hx2, hy] = S.home;
      html += `<g class="k" data-src="ps"><circle cx="${hx2}" cy="${hy}" r="${ds5 ? 15 : 18}" fill="${u('home')}" stroke="#000" stroke-width="1.5"/>
          <circle cx="${hx2}" cy="${hy}" r="${ds5 ? 15 : 18}" fill="none" stroke="var(--acc)" stroke-opacity=".55" stroke-width="1.2"/>
          <path class="nopt" d="M${hx2 - 6} ${hy + 1} l6 -6 l6 6 M${hx2 - 4} ${hy - 1} v6 h8 v-6" fill="none" stroke="#ffffff66" stroke-width="1.4" stroke-linejoin="round"/></g>`;
      if (ds5) html += `<rect class="nopt" x="388" y="${hy + 24}" width="24" height="8" rx="4" fill="${u('btn')}" stroke="#000"/>`;
      if (edge) html += `<rect class="nopt" x="282" y="372" width="36" height="9" rx="4.5" fill="${u('btn')}" stroke="#000"/><rect class="nopt" x="482" y="372" width="36" height="9" rx="4.5" fill="${u('btn')}" stroke="#000"/>`;
      if (ds4) html += `<rect class="nopt" x="378" y="362" width="44" height="9" rx="4" fill="#050506" stroke="${L(k.body, .1)}" stroke-opacity=".4"/>`;
    }
    // sticks
    html += stick(id, k, 'l3', ...S.sticks[0], 'ls', edge) + stick(id, k, 'r3', ...S.sticks[1], 'rs', edge);
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
      c.style.transform = `translate(${(x - 127.5) / 127.5 * 15}px,${(y - 127.5) / 127.5 * 15}px) scale(${pressed ? .93 : 1})`; };
    tilt('ls', S.lx, S.ly, b.l3); tilt('rs', S.rx, S.ry, b.r3);
    for (const t of ['l2', 'r2']) {
      const v = (t === 'l2' ? S.l2v : S.r2v) / 255;
      const g = el.querySelector('#' + id + '-' + t); if (g) g.style.transform = `translateY(${v * 12}px) scaleY(${1 - v * .12})`;
      const p = el.querySelector('#' + id + '-' + t + 'p'); if (p) p.setAttribute('opacity', v * .5);
      const m = el.querySelector('#' + id + '-m-' + t); if (m) m.setAttribute('width', 128 * v);
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
