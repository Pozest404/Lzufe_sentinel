var K = window.K || (window.K = {});

(function (K) {
  'use strict';

  var view = { baseRev: -1, rev: -1, lastHtml: '', loading: false, deferred: false };
  var patchFrame = null;
  var patchActions = [];
  var patchFromRev = -1;
  var fullTimer = null;
  var fullIdle = null;
  var fullPending = false;
  var fullForce = false;
  var externalPending = null;

  function visible() {
    return !document.hidden && !document.querySelector('.workspace').classList.contains('hide-preview');
  }
  function cancelPatch() {
    if (patchFrame != null) {
      if (window.cancelAnimationFrame) window.cancelAnimationFrame(patchFrame);
      else clearTimeout(patchFrame);
    }
    patchFrame = null;
    patchActions = [];
    patchFromRev = -1;
  }
  function cancelFullWork() {
    clearTimeout(fullTimer);
    fullTimer = null;
    if (fullIdle != null) {
      if (window.cancelIdleCallback) window.cancelIdleCallback(fullIdle);
      else clearTimeout(fullIdle);
      fullIdle = null;
    }
  }

  function withBase(html) {
    if (/<head\s*>/i.test(html)) return html.replace(/<head\s*>/i, '<head><base href="/preview/">');
    if (/<head\s[^>]*>/i.test(html)) return html.replace(/<head\s[^>]*>/i, function (tag) { return tag + '<base href="/preview/">'; });
    return '<base href="/preview/">' + html;
  }

  function loadHtml(html, options) {
    if (!html) return false;
    options = options || {};
    var doc = withBase(html);
    if (!options.force && doc === view.lastHtml) {
      view.rev = options.rev;
      view.baseRev = options.baseRev;
      K.store.previewRev = options.rev;
      return false;
    }
    var frame = document.getElementById('previewFrame');
    var scrollY = 0;
    try { scrollY = frame.contentWindow.scrollY || 0; } catch (e) {}
    frame.style.background = K.store.ui.theme === 'light' ? '#eef2ff' : '#060912';
    view.lastHtml = doc;
    view.rev = options.rev;
    view.baseRev = options.baseRev;
    view.loading = true;
    K.store.previewRev = options.rev;
    frame.onload = function () {
      if (view.lastHtml === doc) view.loading = false;
      try { frame.contentWindow.scrollTo(0, scrollY); } catch (e) {}
    };
    frame.srcdoc = doc;
    K.debug.reloads++;
    return true;
  }

  function applyAction(doc, action) {
    var model = K.store.model;
    if (action.type === 'field') {
      if (action.key === 'name') {
        if (!K.render.applyName(doc, model)) throw new Error('name selector missing');
      } else if (action.key === 'desc') {
        if (!K.render.safeMarkup(model.desc) || !K.render.applyDescription(doc, model)) throw new Error('unsafe or missing description');
      } else if (action.key === 'tags') {
        if (!K.render.applyTags(doc, model)) throw new Error('tags selector missing');
      } else if (action.key === 'email') {
        K.render.applyContacts(doc, model, true);
      } else if (action.key === 'seoTitle' || action.key === 'seoDesc') {
        K.render.applySeo(doc, model);
      } else {
        throw new Error('field requires full preview');
      }
      return;
    }
    if (action.type === 'project' || action.type === 'link') {
      K.render.patchProject(doc, action.index, model.projects[action.index], false);
      return;
    }
    if (action.type === 'links') {
      K.render.patchProject(doc, action.index, model.projects[action.index], true);
      return;
    }
    throw new Error('edit requires full preview');
  }

  function runPatch() {
    patchFrame = null;
    if (fullPending || !patchActions.length) return;
    if (document.hidden) return;
    var actions = patchActions;
    var fromRev = patchFromRev;
    patchActions = [];
    patchFromRev = -1;
    if (view.loading || view.baseRev !== K.store.ui.baseRev || view.rev !== fromRev) {
      scheduleFull({ immediate: true });
      return;
    }
    try {
      var doc = document.getElementById('previewFrame').contentDocument;
      if (!doc) throw new Error('preview document unavailable');
      actions.forEach(function (action) { applyAction(doc, action); });
      view.rev = K.store.rev;
      K.store.previewRev = view.rev;
      K.debug.patches++;
    } catch (e) {
      scheduleFull({ immediate: true });
    }
  }

  function queuePatchFrame() {
    if (patchFrame != null || !patchActions.length || fullPending || document.hidden) return;
    if (window.requestAnimationFrame) patchFrame = window.requestAnimationFrame(runPatch);
    else patchFrame = setTimeout(runPatch, 16);
  }

  function schedulePatch(action, fromRev) {
    if (!action || fullPending) return;
    if (!visible()) {
      cancelPatch();
      fullPending = true;
      view.deferred = true;
      return;
    }
    if (!patchActions.length) patchFromRev = fromRev;
    patchActions.push(action);
    queuePatchFrame();
  }

  function runFull() {
    cancelFullWork();
    if (!visible()) {
      view.deferred = true;
      return;
    }
    var html;
    try { html = externalPending != null ? externalPending : K.render.sourceHtml(); }
    catch (e) { fullPending = false; return; }
    var external = externalPending != null;
    externalPending = null;
    var force = fullForce;
    fullPending = false;
    fullForce = false;
    view.deferred = false;
    loadHtml(html, {
      rev: external ? -1 : K.store.rev,
      baseRev: external ? -1 : K.store.ui.baseRev,
      force: force
    });
  }

  function scheduleFull(options) {
    options = options || {};
    externalPending = null;
    fullPending = true;
    if (options.force) fullForce = true;
    cancelPatch();
    cancelFullWork();
    if (options.immediate) {
      runFull();
      return;
    }
    fullTimer = setTimeout(function () {
      fullTimer = null;
      if (window.requestIdleCallback) fullIdle = window.requestIdleCallback(runFull, { timeout: 1000 });
      else fullIdle = setTimeout(runFull, 0);
    }, 400);
  }

  function showExternal(html) {
    cancelPatch();
    cancelFullWork();
    externalPending = html;
    fullPending = true;
    fullForce = true;
    runFull();
  }

  function flush() {
    if (!visible()) return;
    if (fullPending) runFull();
    else queuePatchFrame();
  }

  function refresh() { scheduleFull({ immediate: true, force: true }); }

  K.preview = {
    schedulePatch: schedulePatch,
    scheduleFull: scheduleFull,
    showExternal: showExternal,
    refresh: refresh,
    flush: flush,
    visible: visible,
    getState: function () {
      return { baseRev: view.baseRev, rev: view.rev, loading: view.loading, deferred: view.deferred };
    }
  };
})(K);
