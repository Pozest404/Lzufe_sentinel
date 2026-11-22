(function(){
  'use strict';

  /* ============================ 基础工具 ============================ */
  var FONT = '"PingFang SC","SF Pro Display","Hiragino Sans GB","Microsoft YaHei",system-ui,-apple-system,sans-serif';
  var PHONE_W = 412, PHONE_H = 868;
  var SCREEN_W = 388, SCREEN_H = 844;
  var DETAIL_H = 330;
  var DPR = Math.min(2, window.devicePixelRatio || 1);

  function $(s, r){ return (r || document).querySelector(s); }
  function clamp(v, a, b){ return Math.max(a, Math.min(b, v)); }
  function lerp(a, b, t){ return a + (b - a) * t; }
  function easeOutCubic(t){ return 1 - Math.pow(1 - t, 3); }
  function round2(v){ return Math.round(v * 100) / 100; }

  function hashSeed(str){
    var h = 2166136261; str = String(str);
    for (var i = 0; i < str.length; i++){ h ^= str.charCodeAt(i); h = Math.imul(h, 16777619); }
    return h >>> 0;
  }
  function mulberry32(a){
    return function(){
      a |= 0; a = (a + 0x6D2B79F5) | 0;
      var t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  function hexLerp(a, b, t){
    var na = parseInt(a.slice(1), 16), nb = parseInt(b.slice(1), 16);
    var r = Math.round(lerp((na >> 16) & 255, (nb >> 16) & 255, t));
    var g = Math.round(lerp((na >> 8) & 255, (nb >> 8) & 255, t));
    var bl = Math.round(lerp(na & 255, nb & 255, t));
    return 'rgb(' + r + ',' + g + ',' + bl + ')';
  }
  function withAlpha(hex, a){
    if (hex.charAt(0) !== '#') return hex;
    var n = parseInt(hex.slice(1), 16);
    return 'rgba(' + (n >> 16) + ',' + ((n >> 8) & 255) + ',' + (n & 255) + ',' + a + ')';
  }
  function fmtClock(sec){
    sec = Math.max(0, Math.round(sec));
    var h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
    return String(h).padStart(2, '0') + ':' + String(m).padStart(2, '0') + ':' + String(s).padStart(2, '0');
  }
  function fmtPace(sec){
    sec = Math.max(0, Math.round(sec));
    var m = Math.floor(sec / 60), s = sec % 60;
    return String(m).padStart(2, '0') + "'" + String(s).padStart(2, '0') + '"';
  }
  function fmtDateSlash(){
    var p = state.date.split('-');
    return p[0] + '/' + p[1] + '/' + p[2];
  }
  function addMinutes(hhmm, sec){
    var p = hhmm.split(':').map(Number);
    var total = p[0] * 3600 + p[1] * 60 + Math.round(sec);
    total = ((total % 86400) + 86400) % 86400;
    return String(Math.floor(total / 3600)).padStart(2, '0') + ':' + String(Math.floor((total % 3600) / 60)).padStart(2, '0');
  }
  function rrPath(ctx, x, y, w, h, r){
    r = Math.min(r, w / 2, h / 2);
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.arcTo(x + w, y, x + w, y + h, r);
    ctx.arcTo(x + w, y + h, x, y + h, r);
    ctx.arcTo(x, y + h, x, y, r);
    ctx.arcTo(x, y, x + w, y, r);
    ctx.closePath();
  }

  /* ============================ 地图配色与底图 ============================ */
  var LABELS_BJ = [
    ['二龙间路', .30, .20, 'h'], ['新建宫门路', .22, .37, 'h'], ['昆明湖东路', .055, .56, 'v'],
    ['海淀南路', .55, .70, 'h'], ['知春路', .79, .66, 'h'], ['中关村大街', .66, .48, 'v'],
    ['成府路', .74, .31, 'h'], ['双榆树北路', .73, .83, 'h'], ['彩和坊路', .47, .55, 'v'],
    ['巴沟路', .32, .80, 'h'], ['蓝靛厂北路', .15, .84, 'v'], ['清华西路', .44, .12, 'h'],
    ['中关村北大街', .62, .18, 'v'], ['中关村南路', .73, .55, 'h'], ['双榆树', .82, .90, 'h'],
    ['万泉河路', .035, .36, 'v'], ['苏州街', .41, .34, 'v'], ['万柳中路', .10, .67, 'v'],
    ['北京大学', .15, .10, 'h'], ['清华大学', .53, .055, 'h'], ['人民大学', .57, .94, 'h'],
    ['海淀桥', .36, .63, 'h'], ['北四环西路', .88, .13, 'h']
  ];
  var LABELS_LZ = [
    ['陇桥学院', .42, .10, 'h'], ['博文学院', .07, .07, 'h'], ['兰州工商学院', .66, .17, 'h'],
    ['和平中学', .03, .29, 'h'], ['外语职业学院', .17, .43, 'h'], ['财经大学', .07, .55, 'h'],
    ['财经大学', .07, .60, 'h'], ['兰州财经大学（和平校区）', .59, .60, 'h'], ['财经大学学生公寓', .08, .71, 'h'],
    ['学生公寓楼', .73, .28, 'h'], ['薇乐大道', .28, .37, 'v'],
    ['兰州财经大学', .40, .48, 'h'], ['和平镇', .86, .74, 'h'], ['G309', .56, .90, 'h']
  ];

  function drawAmapBase(ctx, w, h, style, seed, kind){
    var rng = mulberry32(hashSeed(seed + '|' + style + '|' + kind + '|map'));
    var night = style === 'night';
    var trailKind = kind === 'trail';
    var k = w / 388;
    var pal = night ? {
      land:'#171A1F', block:'#1D2127', blockLine:'#242A31',
      roadMinor:'#2B3139', roadMajor:'#4C4327', roadCase:'#20252B',
      park:'#1D2B23', parkLine:'#24382C', tree:'#2C4735', water:'#162630', waterLine:'#1E323D',
      label:'rgba(186,196,190,.42)', halo:'rgba(15,18,22,.85)'
    } : {
      land:'#F7F6F0', block:'#EFEDE5', blockLine:'#E8E5DA',
      roadMinor:'#FFFFFF', roadMajor:'#F5DB9A', roadCase:'#FFFFFF',
      park:'#CFE9C4', parkLine:'#BFE0B2', tree:'#B2DCA2', water:'#BFE2F7', waterLine:'#A7D3EE',
      label:'rgba(140,136,124,.95)', halo:'rgba(255,253,246,.92)'
    };

    ctx.fillStyle = pal.land;
    ctx.fillRect(0, 0, w, h);

    var theta = -0.27 + rng() * 0.08;
    var sx = (trailKind ? 72 : 52) * k, sy = (trailKind ? 62 : 47) * k;
    ctx.save();
    ctx.translate(w / 2, h / 2);
    ctx.rotate(theta);

    // 街区与建筑底块
    for (var bi = -8; bi <= 8; bi++){
      for (var bj = -9; bj <= 9; bj++){
        var bx = bi * sx + 5 * k, by = bj * sy + 5 * k;
        var bw = sx - 10 * k, bh = sy - 10 * k;
        ctx.globalAlpha = 0.42;
        ctx.fillStyle = pal.block;
        rrPath(ctx, bx, by, bw, bh, 1.5 * k);
        ctx.fill();
        var bn = 1 + Math.floor(rng() * 3);
        for (var b = 0; b < bn; b++){
          var sw = (10 + rng() * 20) * k, sh = (8 + rng() * 15) * k;
          var px = bx + 4 * k + rng() * Math.max(2 * k, bw - 8 * k - sw);
          var py = by + 4 * k + rng() * Math.max(2 * k, bh - 8 * k - sh);
          ctx.globalAlpha = 0.45 + rng() * 0.25;
          ctx.fillStyle = rng() < 0.55 ? pal.block : pal.blockLine;
          rrPath(ctx, px, py, sw, sh, 1.2 * k);
          ctx.fill();
        }
      }
    }
    ctx.globalAlpha = 1;

    // 公园绿地
    var parkCount = trailKind ? 1 : (style === 'park' ? 5 : 2);
    for (var pI = 0; pI < parkCount; pI++){
      var px = (w * 0.1 + rng() * w * 0.8) - w / 2;
      var py = (h * 0.1 + rng() * h * 0.8) - h / 2;
      var prx = (38 + rng() * 50) * k, pry = (26 + rng() * 40) * k;
      ctx.fillStyle = pal.park;
      ctx.beginPath();
      ctx.ellipse(px, py, prx, pry, rng() * 1.2, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = pal.parkLine;
      ctx.lineWidth = 1.2 * k;
      ctx.stroke();
      var trees = 10 + Math.floor(rng() * 8);
      for (var t = 0; t < trees; t++){
        var ang = rng() * Math.PI * 2, rad = Math.sqrt(rng());
        ctx.fillStyle = pal.tree;
        ctx.beginPath();
        ctx.arc(px + Math.cos(ang) * prx * rad * 0.86, py + Math.sin(ang) * pry * rad * 0.86, (1.2 + rng() * 1.8) * k, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    // 道路
    function drawMinor(x1, y1, x2, y2, width){
      ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2);
      ctx.strokeStyle = night ? 'rgba(0,0,0,.22)' : 'rgba(226,221,208,.9)';
      ctx.lineWidth = (width + 1.6) * k; ctx.stroke();
      ctx.strokeStyle = pal.roadMinor; ctx.lineWidth = width * k; ctx.stroke();
    }
    function vlines(major){
      for (var i = -10; i <= 10; i++){
        var x = i * sx;
        var isMajor = !trailKind && (i % 4 === 0);
        if (isMajor !== major) continue;
        ctx.beginPath();
        ctx.moveTo(x, -h * 1.3);
        ctx.lineTo(x, h * 1.3);
        if (major){
          ctx.strokeStyle = pal.roadCase; ctx.lineWidth = 12 * k;
        } else {
          ctx.strokeStyle = night ? 'rgba(0,0,0,.22)' : 'rgba(226,221,208,.9)'; ctx.lineWidth = 5.2 * k;
        }
        ctx.stroke();
        if (!major){
          ctx.strokeStyle = pal.roadMinor; ctx.lineWidth = 3.8 * k; ctx.stroke();
        }
      }
    }
    function hlines(major){
      for (var j = -11; j <= 11; j++){
        var y = j * sy;
        var isMajor = !trailKind && (j % 4 === 2);
        if (isMajor !== major) continue;
        ctx.beginPath();
        ctx.moveTo(-w * 1.3, y);
        ctx.lineTo(w * 1.3, y);
        if (major){
          ctx.strokeStyle = pal.roadCase; ctx.lineWidth = 12 * k;
        } else {
          ctx.strokeStyle = night ? 'rgba(0,0,0,.22)' : 'rgba(226,221,208,.9)'; ctx.lineWidth = 5.2 * k;
        }
        ctx.stroke();
        if (!major){
          ctx.strokeStyle = pal.roadMinor; ctx.lineWidth = 3.8 * k; ctx.stroke();
        }
      }
    }
    // 细支路网格
    if (!trailKind){
      ctx.globalAlpha = night ? 0.4 : 0.85;
      for (var ai = -20; ai <= 20; ai++){
        drawMinor(ai * sx + sx / 2, -h * 1.3, ai * sx + sx / 2, h * 1.3, 1.7);
      }
      for (var aj = -22; aj <= 22; aj++){
        drawMinor(-w * 1.3, aj * sy + sy / 2, w * 1.3, aj * sy + sy / 2, 1.7);
      }
      ctx.globalAlpha = 1;
    }

    vlines(false); hlines(false);
    ctx.lineCap = 'round';
    vlines(true); hlines(true);
    // 主干道黄色路面
    for (var mi = -10; mi <= 10; mi++){
      if (mi % 4 !== 0) continue;
      var mx = mi * sx;
      ctx.beginPath(); ctx.moveTo(mx, -h * 1.3); ctx.lineTo(mx, h * 1.3);
      ctx.strokeStyle = pal.roadMajor; ctx.lineWidth = 7.4 * k; ctx.stroke();
    }
    for (var mj = -11; mj <= 11; mj++){
      if (mj % 4 !== 2) continue;
      var my = mj * sy;
      ctx.beginPath(); ctx.moveTo(-w * 1.3, my); ctx.lineTo(w * 1.3, my);
      ctx.strokeStyle = pal.roadMajor; ctx.lineWidth = 7.4 * k; ctx.stroke();
    }
    // 斜向城市主干道
    ctx.beginPath();
    ctx.moveTo(-w * 0.25, h * 1.25);
    ctx.bezierCurveTo(w * 0.3, h * 0.82, w * 0.58, h * 0.44, w * 1.25, h * 0.08);
    ctx.strokeStyle = pal.roadCase; ctx.lineWidth = 14 * k; ctx.stroke();
    ctx.strokeStyle = pal.roadMajor; ctx.lineWidth = 8.6 * k; ctx.stroke();
    // 斜向支路
    ctx.beginPath();
    ctx.moveTo(-w * 0.05, -h * 0.2);
    ctx.bezierCurveTo(w * 0.3, h * 0.18, w * 0.55, h * 0.42, w * 0.95, h * 1.2);
    ctx.strokeStyle = night ? 'rgba(0,0,0,.2)' : 'rgba(226,221,208,.85)';
    ctx.lineWidth = 6 * k; ctx.stroke();
    ctx.strokeStyle = pal.roadMinor; ctx.lineWidth = 4.4 * k; ctx.stroke();

    // 水系最后绘制，避免道路穿过水面
    if (!trailKind && (style !== 'park' || rng() < 0.55)){
      var wcx = -w * 0.40, wcy = h * 0.02;
      ctx.fillStyle = pal.water;
      ctx.beginPath();
      ctx.ellipse(wcx, wcy, w * (0.13 + rng() * 0.04), h * (0.16 + rng() * 0.06), 0.25, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = pal.waterLine;
      ctx.lineWidth = 1.2 * k;
      ctx.stroke();
      ctx.globalAlpha = 0.5;
      ctx.strokeStyle = night ? 'rgba(120,180,210,.25)' : 'rgba(255,255,255,.75)';
      ctx.lineWidth = 0.9 * k;
      for (var wv = 0; wv < 3; wv++){
        var wx = -w * 0.44 + rng() * w * 0.14;
        var wy = wcy - h * 0.12 + rng() * h * 0.24;
        ctx.beginPath();
        ctx.moveTo(wx, wy);
        ctx.lineTo(wx + 16 * k, wy + 1.5 * k);
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
    }
    ctx.restore();

    // 路名
    var labels = kind === 'trail' ? LABELS_LZ : LABELS_BJ;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.font = '500 ' + Math.max(10.5, 12.2 * k) + 'px ' + FONT;
    for (var li = 0; li < labels.length; li++){
      var lb = labels[li];
      ctx.save();
      ctx.translate(lb[1] * w, lb[2] * h);
      ctx.rotate(theta + (lb[3] === 'v' ? Math.PI / 2 : 0));
      ctx.lineWidth = 3 * k;
      ctx.strokeStyle = pal.halo;
      ctx.strokeText(lb[0], 0, 0);
      ctx.fillStyle = pal.label;
      ctx.fillText(lb[0], 0, 0);
      ctx.restore();
    }
    ctx.textAlign = 'left';
    ctx.textBaseline = 'alphabetic';
  }

  /* ============================ 轨迹生成 ============================ */
  function genLoop(rng){
    // 参考 Keep 轨迹：左下起跑，一段蜿蜒的连接线，接入不规则环线
    var pts = [];
    var startX = -0.62 - rng() * 0.18;
    var startY = 1.02 + rng() * 0.08;
    var cx = 0.10 + (rng() - 0.5) * 0.24;
    var cy = -0.22 + (rng() - 0.5) * 0.18;
    var R = 0.54 + rng() * 0.12;
    var stemEnd = [cx + R * 0.12, cy + R * 0.98];
    var Ns = 70;
    for (var s = 0; s <= Ns; s++){
      var u = s / Ns;
      pts.push([
        lerp(startX, stemEnd[0], u) + Math.sin(u * Math.PI * 2.4) * 0.14 * (1 - u * 0.35) + Math.sin(u * Math.PI * 5.2 + 1.3) * 0.035 * u,
        lerp(startY, stemEnd[1], u)
      ]);
    }
    var startA = Math.PI / 2 + (rng() - 0.5) * 0.2;
    var turns = 1.55 + rng() * 0.55;
    var p1 = rng() * Math.PI * 2, p2 = rng() * Math.PI * 2;
    var Nl = 280;
    for (var i = 0; i <= Nl; i++){
      var a = startA + i / Nl * Math.PI * 2 * turns;
      var r = R * (1 + 0.15 * Math.sin(3 * a + p1) + 0.09 * Math.sin(5 * a + p2) + 0.035 * Math.sin(9 * a + p1 * 1.7));
      pts.push([cx + Math.cos(a) * r * 1.18, cy + Math.sin(a) * r * 0.84]);
    }
    return pts;
  }
  function genOutBack(rng){
    var forward = [], N = 120;
    var amp = 0.10 + rng() * 0.13, freq = 1.1 + rng() * 1.6;
    var phase = rng() * Math.PI * 2, bend = 0.12 + rng() * 0.22;
    for (var i = 0; i <= N; i++){
      var u = i / N, x = -0.9 + u * 1.8;
      var y = Math.sin(u * Math.PI * freq + phase) * amp + Math.sin(u * Math.PI) * bend + Math.sin(u * Math.PI * 2) * 0.03;
      forward.push([x, y]);
    }
    var back = [];
    for (var j = forward.length - 1; j >= 0; j--){
      var off = 0.055 + 0.018 * Math.sin(j * 0.23);
      back.push([forward[j][0] + 0.01, forward[j][1] + off]);
    }
    return forward.concat(back);
  }
  function genTrack(rng){
    var pts = [], laps = 3 + Math.floor(rng() * 2), per = 200;
    var a = 0.80, b = 0.50, expo = 2 / 3.4;
    for (var l = 0; l < laps; l++){
      var off = l * 0.022;
      for (var i = 0; i < per; i++){
        var t = i / per * Math.PI * 2, ct = Math.cos(t), st = Math.sin(t);
        pts.push([
          (a + off) * Math.sign(ct) * Math.pow(Math.abs(ct), expo),
          (b + off) * Math.sign(st) * Math.pow(Math.abs(st), expo)
        ]);
      }
    }
    return pts;
  }
  function genCity(rng){
    var n = 5 + Math.floor(rng() * 4), g = 0.22;
    var baseR = 0.58 + rng() * 0.24, startA = rng() * Math.PI * 2;
    var corners = [];
    for (var i = 0; i < n; i++){
      var a = startA + i / n * Math.PI * 2 + (rng() - 0.5) * 0.36;
      var r = baseR * (0.78 + rng() * 0.42);
      corners.push([Math.round(Math.cos(a) * r / g) * g, Math.round(Math.sin(a) * r * 0.86 / g) * g]);
    }
    var pts = [];
    for (var j = 0; j < corners.length; j++){
      var c1 = corners[j], c2 = corners[(j + 1) % corners.length];
      pts.push([c1[0], c1[1]]);
      var corner = rng() < 0.5 ? [c2[0], c1[1]] : [c1[0], c2[1]];
      var prev = pts[pts.length - 1];
      if (Math.abs(corner[0] - prev[0]) > 1e-6 || Math.abs(corner[1] - prev[1]) > 1e-6) pts.push(corner);
    }
    var clean = [];
    for (var m = 0; m < pts.length; m++){
      var p = pts[m], q = clean[clean.length - 1];
      if (!q || Math.abs(p[0] - q[0]) > 1e-6 || Math.abs(p[1] - q[1]) > 1e-6) clean.push(p);
    }
    return clean;
  }
  function catmullRom(pts, closed){
    if (pts.length < 3) return pts.slice();
    var P = closed ? [pts[pts.length - 1]].concat(pts, [pts[0], pts[1]]) : [pts[0]].concat(pts, [pts[pts.length - 1]]);
    var out = [], segs = closed ? pts.length : pts.length - 1, steps = 8;
    for (var i = 0; i < segs; i++){
      var p0 = P[i], p1 = P[i + 1], p2 = P[i + 2], p3 = P[i + 3];
      for (var j = 0; j < steps; j++){
        var t = j / steps, t2 = t * t, t3 = t2 * t;
        out.push([
          0.5 * (2 * p1[0] + (-p0[0] + p2[0]) * t + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3),
          0.5 * (2 * p1[1] + (-p0[1] + p2[1]) * t + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3)
        ]);
      }
    }
    out.push(closed ? out[0].slice() : pts[pts.length - 1].slice());
    return out;
  }
  function buildRoute(shape, rng){
    var raw;
    if (shape === 'loop') raw = genLoop(rng);
    else if (shape === 'outback') raw = genOutBack(rng);
    else if (shape === 'track') raw = genTrack(rng);
    else raw = genCity(rng);
    return catmullRom(raw, shape === 'city');
  }
  function fitPoints(pts, w, h, pad){
    var minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (var i = 0; i < pts.length; i++){
      var p = pts[i];
      if (p[0] < minX) minX = p[0];
      if (p[0] > maxX) maxX = p[0];
      if (p[1] < minY) minY = p[1];
      if (p[1] > maxY) maxY = p[1];
    }
    var bw = Math.max(1e-6, maxX - minX), bh = Math.max(1e-6, maxY - minY);
    var s = Math.min((w - pad * 2) / bw, (h - pad * 2) / bh);
    var cx = (minX + maxX) / 2, cy = (minY + maxY) / 2, out = [];
    for (var j = 0; j < pts.length; j++){
      out.push([w / 2 + (pts[j][0] - cx) * s, h / 2 + (pts[j][1] - cy) * s]);
    }
    return out;
  }
  function cumulative(pts){
    var L = [0];
    for (var i = 1; i < pts.length; i++) L.push(L[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
    return L;
  }
  function makeScreen(norm, w, h, pad){
    var pts = fitPoints(norm, w, h, pad);
    var L = cumulative(pts);
    return { pts: pts, L: L, total: L[L.length - 1] || 1 };
  }
  function findAt(pts, L, target){
    if (target <= 0) return pts[0];
    if (target >= L[L.length - 1]) return pts[pts.length - 1];
    var lo = 0, hi = L.length - 1;
    while (lo < hi - 1){ var mid = (lo + hi) >> 1; if (L[mid] <= target) lo = mid; else hi = mid; }
    var segLen = L[hi] - L[lo] || 1, t = (target - L[lo]) / segLen;
    return [lerp(pts[lo][0], pts[hi][0], t), lerp(pts[lo][1], pts[hi][1], t)];
  }
  function sliceHead(pts, L, target){
    var head = [];
    for (var i = 0; i < pts.length; i++){
      if (L[i] <= target) head.push(pts[i]); else break;
    }
    var idx = head.length - 1;
    if (idx >= 0 && idx < pts.length - 1){
      var segLen = L[idx + 1] - L[idx] || 1;
      var t = clamp((target - L[idx]) / segLen, 0, 1);
      head.push([lerp(pts[idx][0], pts[idx + 1][0], t), lerp(pts[idx][1], pts[idx + 1][1], t)]);
    }
    return head;
  }

  /* ============================ 轨迹绘制 ============================ */
  function routeColor(t){
    t = clamp(t, 0, 1);
    if (t < 0.42) return hexLerp('#43C89B', '#4FD3AB', t / 0.42);
    return hexLerp('#4FD3AB', '#45D6E6', (t - 0.42) / 0.58);
  }
  function strokePath(ctx, pts){
    ctx.beginPath();
    ctx.moveTo(pts[0][0], pts[0][1]);
    for (var i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0], pts[i][1]);
    ctx.stroke();
  }
  function drawRoute(ctx, screen, progress, opt){
    var head = sliceHead(screen.pts, screen.L, screen.total * clamp(progress, 0, 1));
    if (head.length < 2) return;
    var width = opt.width;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    ctx.strokeStyle = opt.casing || 'rgba(255,255,255,.95)';
    ctx.lineWidth = width + (opt.casingW || 4.5);
    strokePath(ctx, head);

    var step = Math.max(2, Math.floor(head.length / 160));
    for (var i = 0; i < head.length - 1; i += step){
      var j = Math.min(head.length - 1, i + step);
      ctx.beginPath();
      ctx.moveTo(head[i][0], head[i][1]);
      for (var m = i + 1; m <= j; m++) ctx.lineTo(head[m][0], head[m][1]);
      ctx.strokeStyle = routeColor((screen.L[i] || 0) / screen.total);
      ctx.lineWidth = width;
      ctx.stroke();
    }

    if (opt.markers){
      var start = screen.pts[0];
      ctx.beginPath();
      ctx.arc(start[0], start[1], opt.marker * 1.16, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(255,255,255,.95)';
      ctx.fill();
      ctx.beginPath();
      ctx.arc(start[0], start[1], opt.marker, 0, Math.PI * 2);
      ctx.fillStyle = '#57CDB0';
      ctx.fill();
      ctx.fillStyle = '#FFFFFF';
      ctx.font = '700 ' + (opt.marker * 0.92).toFixed(1) + 'px ' + FONT;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText((state.user || 'K').charAt(0), start[0], start[1] + 0.5);
      ctx.textAlign = 'left';
      ctx.textBaseline = 'alphabetic';

      if (progress > 0.995){
        var end = screen.pts[screen.pts.length - 1];
        ctx.beginPath();
        ctx.arc(end[0], end[1], opt.marker * 1.15, 0, Math.PI * 2);
        ctx.fillStyle = '#FFFFFF';
        ctx.fill();
        ctx.strokeStyle = 'rgba(40,40,40,.12)';
        ctx.lineWidth = 1;
        ctx.stroke();
        ctx.strokeStyle = '#2A2A28';
        ctx.lineWidth = 2 * (opt.scale || 1);
        ctx.beginPath();
        ctx.moveTo(end[0] - opt.marker * 0.28, end[1] + opt.marker * 0.62);
        ctx.lineTo(end[0] - opt.marker * 0.28, end[1] - opt.marker * 0.62);
        ctx.stroke();
        ctx.fillStyle = '#16B77D';
        ctx.beginPath();
        ctx.moveTo(end[0] - opt.marker * 0.26, end[1] - opt.marker * 0.62);
        ctx.lineTo(end[0] + opt.marker * 0.58, end[1] - opt.marker * 0.30);
        ctx.lineTo(end[0] - opt.marker * 0.26, end[1] - opt.marker * 0.02);
        ctx.closePath();
        ctx.fill();
      }
    }

    if (opt.pulse && progress < 0.999){
      var hp = head[head.length - 1];
      var pulse = (Math.sin((opt.now || 0) / 240) + 1) / 2;
      ctx.beginPath();
      ctx.arc(hp[0], hp[1], (7 + 7 * pulse) * (opt.scale || 1), 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(69,215,214,' + (0.22 * (1 - pulse)).toFixed(2) + ')';
      ctx.fill();
      ctx.beginPath();
      ctx.arc(hp[0], hp[1], 4.4 * (opt.scale || 1), 0, Math.PI * 2);
      ctx.fillStyle = '#FFFFFF';
      ctx.fill();
      ctx.strokeStyle = '#3CCDB0';
      ctx.lineWidth = 2.2 * (opt.scale || 1);
      ctx.stroke();
    }
  }

  /* ============================ 状态与数据 ============================ */
  var state = {
    shape: 'loop', distance: 1.5, pace: 373, style: 'city',
    seed: 'keep-trail-2026', user: '儒雅的_Keeperze4b', location: '兰州市',
    weather: '晴', temp: 12, date: '2026-11-09', time: '16:29', view: 'detail'
  };
  var stats = null, routeNorm = null, detailScreen = null, trailScreen = null;
  var detailCanvas, detailCtx, trailCanvas, trailCtx;
  var baseCache = {};
  var animating = false, animStart = 0, animDur = 2100, animProg = 1, rafId = 0;

  function computeStats(){
    var rng = mulberry32(hashSeed(state.seed + '|stats'));
    var distance = round2(state.distance + rng() * 0.12);
    var pace = clamp(state.pace + Math.round((rng() - 0.5) * 6), 180, 600);
    var duration = Math.round(distance * pace);
    var calories = Math.round(distance * 64.6);
    var total = Math.round(duration * 1.72);
    var load = Math.round(distance * 17 + duration / 60 * 1.1);
    var elevation = Math.max(1, Math.round(distance * 7 + rng() * 2));
    return { distance: distance, pace: pace, duration: duration, calories: calories, total: total, load: load, elevation: elevation };
  }

  /* ============================ 画布 ============================ */
  function initCanvases(){
    detailCanvas = $('#detailCanvas');
    trailCanvas = $('#trailCanvas');
    detailCanvas.width = Math.round(SCREEN_W * DPR);
    detailCanvas.height = Math.round(DETAIL_H * DPR);
    trailCanvas.width = Math.round(SCREEN_W * DPR);
    trailCanvas.height = Math.round(SCREEN_H * DPR);
    detailCtx = detailCanvas.getContext('2d');
    trailCtx = trailCanvas.getContext('2d');
  }

  function getBase(w, h, kind){
    var key = state.style + '|' + state.seed + '|' + kind + '|' + w + 'x' + h;
    if (baseCache[key]) return baseCache[key];
    var cv = document.createElement('canvas');
    cv.width = Math.round(w * DPR);
    cv.height = Math.round(h * DPR);
    var ctx = cv.getContext('2d');
    ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
    drawAmapBase(ctx, w, h, state.style, state.seed, kind);
    baseCache[key] = cv;
    return cv;
  }

  function rebuildScreens(){
    detailScreen = makeScreen(routeNorm, SCREEN_W, DETAIL_H, 46);
    trailScreen = makeScreen(routeNorm, SCREEN_W, SCREEN_H, 34);
  }

  function drawDetail(now){
    if (!detailCtx) return;
    detailCtx.setTransform(DPR, 0, 0, DPR, 0, 0);
    detailCtx.clearRect(0, 0, SCREEN_W, DETAIL_H);
    detailCtx.drawImage(getBase(SCREEN_W, DETAIL_H, 'detail'), 0, 0, SCREEN_W, DETAIL_H);
    drawRoute(detailCtx, detailScreen, animProg, {
      width: 4.8, casingW: 4.6, casing: 'rgba(255,255,255,.92)',
      scale: 1, markers: false, pulse: animating, now: now
    });
  }

  function drawTrail(now){
    if (!trailCtx) return;
    trailCtx.setTransform(DPR, 0, 0, DPR, 0, 0);
    trailCtx.clearRect(0, 0, SCREEN_W, SCREEN_H);
    trailCtx.drawImage(getBase(SCREEN_W, SCREEN_H, 'trail'), 0, 0, SCREEN_W, SCREEN_H);
    drawRoute(trailCtx, trailScreen, animProg, {
      width: 7, casingW: 6.2, casing: 'rgba(255,255,255,.95)',
      scale: 1, markers: true, marker: 14, pulse: animating, now: now
    });
  }

  function drawAll(now){ drawDetail(now); drawTrail(now); }

  /* ============================ 动画 ============================ */
  function playIntro(){
    animProg = 0;
    animating = true;
    animStart = performance.now();
    animDur = 1900 + Math.random() * 400;
    if (!rafId) rafId = requestAnimationFrame(tick);
  }
  function tick(now){
    if (animating){
      var t = clamp((now - animStart) / animDur, 0, 1);
      animProg = easeOutCubic(t);
      if (t >= 1){ animating = false; animProg = 1; }
    }
    drawAll(now);
    rafId = animating ? requestAnimationFrame(tick) : 0;
  }

  /* ============================ 文本 ============================ */
  function renderText(){
    var end = addMinutes(state.time, stats.total);
    $('#distNum').textContent = stats.distance.toFixed(2);
    $('#userName').textContent = state.user;
    $('#avatarLetter').textContent = (state.user || 'K').charAt(0);
    $('#metaText').textContent = fmtDateSlash() + ' ' + state.time + ' - ' + end + ' ' +
      state.location + ' · ' + state.weather + ' · ' + state.temp + '°C';
    $('#statTime').textContent = fmtClock(stats.duration);
    $('#statPace').textContent = fmtPace(stats.pace);
    $('#statCal').innerHTML = stats.calories + '<small>千卡</small>';
    $('#statTotal').textContent = fmtClock(stats.total);
    $('#statLoad').textContent = stats.load;
    $('#statElev').innerHTML = stats.elevation + '<small>米</small>';
  }

  /* ============================ 生成与同步 ============================ */
  function regenerate(animate){
    var rng = mulberry32(hashSeed(state.seed + '|' + state.shape));
    routeNorm = buildRoute(state.shape, rng);
    stats = computeStats();
    baseCache = {};
    rebuildScreens();
    renderText();
    if (animate && $('#animToggle').checked){
      playIntro();
    } else {
      animating = false;
      animProg = 1;
      drawAll(performance.now());
    }
  }

  function setFill(el){
    var min = Number(el.min), max = Number(el.max), v = Number(el.value);
    el.style.setProperty('--fill', (((v - min) / (max - min)) * 100).toFixed(2) + '%');
  }

  function syncControls(){
    var btns = $('#viewChips').querySelectorAll('.chip');
    for (var i = 0; i < btns.length; i++) btns[i].classList.toggle('active', btns[i].getAttribute('data-v') === state.view);
    var sc = $('#shapeChips').querySelectorAll('.chip');
    for (var j = 0; j < sc.length; j++) sc[j].classList.toggle('active', sc[j].getAttribute('data-v') === state.shape);
    var st = $('#styleChips').querySelectorAll('.chip');
    for (var k = 0; k < st.length; k++) st[k].classList.toggle('active', st[k].getAttribute('data-v') === state.style);

    var dr = $('#distRange');
    dr.value = state.distance; setFill(dr);
    $('#distLabel').textContent = Number(state.distance).toFixed(1) + ' 公里';
    var pr = $('#paceRange');
    pr.value = state.pace; setFill(pr);
    $('#paceLabel').textContent = fmtPace(state.pace);

    $('#seedInput').value = state.seed;
    $('#userInput').value = state.user;
    $('#locInput').value = state.location;
    $('#weatherSelect').value = state.weather;
    $('#tempInput').value = state.temp;
    $('#dateInput').value = state.date;
    $('#timeInput').value = state.time;
  }

  function setView(v){
    state.view = v;
    $('#viewDetail').hidden = v !== 'detail';
    $('#viewTrail').hidden = v !== 'trail';
    var btns = $('#viewChips').querySelectorAll('.chip');
    for (var i = 0; i < btns.length; i++) btns[i].classList.toggle('active', btns[i].getAttribute('data-v') === v);
    drawAll(performance.now());
  }

  function randomizeAll(){
    var shapes = ['loop', 'outback', 'track', 'city'], weathers = ['多云', '晴', '阴', '微风', '小雨'];
    state.shape = shapes[Math.floor(Math.random() * shapes.length)];
    state.distance = Math.round((0.8 + Math.random() * 7) * 10) / 10;
    state.pace = 265 + Math.round(Math.random() * 120);
    state.seed = 'keep-' + Math.random().toString(36).slice(2, 8);
    state.weather = weathers[Math.floor(Math.random() * weathers.length)];
    state.temp = 8 + Math.round(Math.random() * 17);
    syncControls();
    regenerate(true);
  }

  /* ============================ 导出海报 ============================ */
  function renderPoster(){
    var W = 1080, H = 1440;
    var cv = document.createElement('canvas');
    cv.width = W; cv.height = H;
    var ctx = cv.getContext('2d');

    var bg = ctx.createLinearGradient(0, 0, W, H);
    bg.addColorStop(0, '#FFFFFF');
    bg.addColorStop(1, '#FBFCFA');
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, W, H);
    var glow = ctx.createRadialGradient(W * 0.85, -60, 0, W * 0.85, -60, 760);
    glow.addColorStop(0, 'rgba(34,209,131,.10)');
    glow.addColorStop(1, 'rgba(34,209,131,0)');
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, W, H);

    var lg = ctx.createLinearGradient(64, 78, 136, 150);
    lg.addColorStop(0, '#2BE38F'); lg.addColorStop(1, '#0FA968');
    ctx.fillStyle = lg;
    rrPath(ctx, 64, 78, 72, 72, 22);
    ctx.fill();
    ctx.fillStyle = '#05130D';
    ctx.font = '900 40px ' + FONT;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('K', 100, 116);
    ctx.textAlign = 'left';
    ctx.textBaseline = 'alphabetic';

    ctx.fillStyle = '#111111';
    ctx.font = '700 46px ' + FONT;
    ctx.fillText('Keep 运动轨迹', 162, 122);
    ctx.fillStyle = 'rgba(0,0,0,.42)';
    ctx.font = '400 23px ' + FONT;
    ctx.fillText(state.user + '  ·  ' + fmtDateSlash() + ' ' + state.time, 162, 158);

    // 地图
    var mx = 64, my = 200, mw = 952, mh = 690, mr = 36;
    ctx.save();
    rrPath(ctx, mx, my, mw, mh, mr);
    ctx.clip();
    ctx.translate(mx, my);
    drawAmapBase(ctx, mw, mh, state.style, state.seed, 'trail');
    drawRoute(ctx, makeScreen(routeNorm, mw, mh, 66), 1, {
      width: 10, casingW: 9, casing: 'rgba(255,255,255,.95)',
      scale: mw / 388, markers: true, marker: 26, pulse: false, now: 0
    });
    ctx.restore();
    ctx.save();
    rrPath(ctx, mx, my, mw, mh, mr);
    ctx.strokeStyle = 'rgba(0,0,0,.08)';
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.restore();

    ctx.fillStyle = '#111111';
    ctx.font = '600 30px ' + FONT;
    ctx.fillText('户外跑步', 64, 952);
    ctx.fillStyle = 'rgba(0,0,0,.42)';
    ctx.font = '400 23px ' + FONT;
    ctx.fillText(state.location + '  ·  ' + state.weather + ' ' + state.temp + '°C', 64, 990);

    var cols = [64, 406, 714];
    var labels = ['距离', '训练时长', '平均配速'];
    var values = [stats.distance.toFixed(2), fmtClock(stats.duration), fmtPace(stats.pace)];
    var units = ['公里', '', ''];
    for (var i = 0; i < 3; i++){
      ctx.fillStyle = 'rgba(0,0,0,.40)';
      ctx.font = '500 21px ' + FONT;
      ctx.fillText(labels[i], cols[i], 1052);
      ctx.fillStyle = '#111111';
      ctx.font = '800 58px ' + FONT;
      ctx.fillText(values[i], cols[i], 1120);
      if (units[i]){
        var vw = ctx.measureText(values[i]).width;
        ctx.fillStyle = 'rgba(0,0,0,.42)';
        ctx.font = '600 22px ' + FONT;
        ctx.fillText(units[i], cols[i] + vw + 10, 1120);
      }
    }

    ctx.strokeStyle = 'rgba(0,0,0,.08)';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(64, 1176);
    ctx.lineTo(1016, 1176);
    ctx.stroke();

    var metrics = [
      ['运动消耗', stats.calories, '千卡'],
      ['总时长', fmtClock(stats.total), ''],
      ['运动负荷', stats.load, ''],
      ['爬升高度', stats.elevation, '米']
    ];
    for (var m = 0; m < 4; m++){
      var x = 64 + m * 238, mv = metrics[m];
      ctx.fillStyle = 'rgba(0,0,0,.40)';
      ctx.font = '500 20px ' + FONT;
      ctx.fillText(mv[0], x, 1226);
      ctx.fillStyle = '#111111';
      ctx.font = '700 30px ' + FONT;
      ctx.fillText(String(mv[1]), x, 1270);
      if (mv[2]){
        var w2 = ctx.measureText(String(mv[1])).width;
        ctx.fillStyle = 'rgba(0,0,0,.38)';
        ctx.font = '500 17px ' + FONT;
        ctx.fillText(mv[2], x + w2 + 6, 1270);
      }
    }

    ctx.fillStyle = 'rgba(0,0,0,.38)';
    ctx.font = '400 19px ' + FONT;
    ctx.fillText(state.user + '  ·  ' + state.location, 64, 1370);
    ctx.textAlign = 'right';
    ctx.fillStyle = 'rgba(34,170,110,.85)';
    ctx.fillText('Keep · 自律给我自由', 1016, 1370);
    ctx.textAlign = 'left';

    return cv;
  }

  var posterBlobUrl = null;

  function exportPoster(){
    try {
      var cv = renderPoster();
      cv.toBlob(function(blob){
        if (!blob){ showToast('导出失败，请重试'); return; }
        if (posterBlobUrl) URL.revokeObjectURL(posterBlobUrl);
        posterBlobUrl = URL.createObjectURL(blob);
        $('#posterImg').src = posterBlobUrl;
        $('#posterModal').classList.add('show');
      }, 'image/png');
    } catch (e){
      showToast('导出失败，请重试');
    }
  }
  function downloadPoster(){
    if (!posterBlobUrl) return;
    var a = document.createElement('a');
    a.href = posterBlobUrl;
    a.download = 'keep-trail-' + state.seed.replace(/[^\w-]/g, '').slice(0, 16) + '.png';
    document.body.appendChild(a);
    a.click();
    a.remove();
    showToast('轨迹图片已下载');
  }
  function closePoster(){ $('#posterModal').classList.remove('show'); }

  var toastTimer = 0;
  function showToast(msg){
    var t = $('#toast');
    t.textContent = msg;
    t.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function(){ t.classList.remove('show'); }, 2200);
  }

  /* ============================ 事件 ============================ */
  function bind(){
    $('#viewChips').addEventListener('click', function(e){
      var btn = e.target.closest('.chip');
      if (btn) setView(btn.getAttribute('data-v'));
    });
    $('#shapeChips').addEventListener('click', function(e){
      var btn = e.target.closest('.chip');
      if (!btn) return;
      state.shape = btn.getAttribute('data-v');
      syncControls();
      regenerate(true);
    });
    $('#styleChips').addEventListener('click', function(e){
      var btn = e.target.closest('.chip');
      if (!btn) return;
      state.style = btn.getAttribute('data-v');
      syncControls();
      regenerate(true);
    });
    $('#distRange').addEventListener('input', function(e){
      state.distance = Number(e.target.value);
      setFill(e.target);
      $('#distLabel').textContent = state.distance.toFixed(1) + ' 公里';
      regenerate(false);
    });
    $('#paceRange').addEventListener('input', function(e){
      state.pace = Number(e.target.value);
      setFill(e.target);
      $('#paceLabel').textContent = fmtPace(state.pace);
      regenerate(false);
    });
    $('#seedInput').addEventListener('change', function(e){
      state.seed = e.target.value.trim() || 'keep-trail-2026';
      regenerate(true);
    });
    $('#diceBtn').addEventListener('click', function(){
      state.seed = 'keep-' + Math.random().toString(36).slice(2, 8);
      syncControls();
      regenerate(true);
    });
    $('#userInput').addEventListener('input', function(e){
      state.user = e.target.value || 'Keep User';
      renderText();
      drawAll(performance.now());
    });
    $('#locInput').addEventListener('input', function(e){
      state.location = e.target.value || '未命名地点';
      renderText();
    });
    $('#weatherSelect').addEventListener('change', function(e){
      state.weather = e.target.value;
      renderText();
    });
    $('#tempInput').addEventListener('input', function(e){
      state.temp = Number(e.target.value) || 0;
      renderText();
    });
    $('#dateInput').addEventListener('change', function(e){
      if (e.target.value){ state.date = e.target.value; renderText(); }
    });
    $('#timeInput').addEventListener('change', function(e){
      if (e.target.value){ state.time = e.target.value; renderText(); }
    });

    $('#regenBtn').addEventListener('click', randomizeAll);
    $('#replayBtn').addEventListener('click', function(){
      playIntro();
      showToast('正在重播轨迹动画');
    });
    $('#exportBtn').addEventListener('click', exportPoster);

    $('#modalClose').addEventListener('click', closePoster);
    $('#modalCancel').addEventListener('click', closePoster);
    $('#modalDownload').addEventListener('click', downloadPoster);
    $('#posterModal').addEventListener('click', function(e){
      if (e.target === $('#posterModal')) closePoster();
    });
    document.addEventListener('keydown', function(e){
      if (e.key === 'Escape') closePoster();
    });
  }

  /* ============================ 自适应 ============================ */
  function fitPhone(){
    var stage = $('.stage'), holder = $('#phoneHolder'), phone = $('#phone');
    var availH = window.innerHeight - 110;
    var availW = stage.clientWidth - 48;
    if (window.innerWidth <= 980){
      availH = 1000;
      availW = Math.min(window.innerWidth - 32, 560);
    }
    var s = Math.min(1, availW / PHONE_W, availH / PHONE_H);
    s = Math.max(0.4, s);
    phone.style.transform = 'scale(' + s + ')';
    holder.style.width = (PHONE_W * s) + 'px';
    holder.style.height = (PHONE_H * s) + 'px';
  }

  /* ============================ 初始化 ============================ */
  function init(){
    state.date = $('#dateInput').value || state.date;
    state.time = $('#timeInput').value || state.time;

    initCanvases();
    routeNorm = buildRoute(state.shape, mulberry32(hashSeed(state.seed + '|' + state.shape)));
    stats = computeStats();
    rebuildScreens();
    syncControls();
    renderText();
    animProg = 1;
    drawAll(performance.now());

    bind();
    fitPhone();
    window.addEventListener('resize', fitPhone);
    setTimeout(fitPhone, 60);

    if ($('#animToggle').checked) playIntro();
  }

  if (document.readyState === 'loading'){
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
