var K = window.K || (window.K = {});

(function (K) {
  'use strict';

  K.debug = { builds: 0, patches: 0, reloads: 0 };

  var store = K.store = {
    baseHtml: '',
    model: null,
    rev: 0,
    savedRev: 0,
    previewRev: -1,
    rawMode: false,
    history: { items: [], index: -1, timer: null },
    ui: {
      tab: 'basic', theme: 'dark', autoPublish: true,
      collapsed: new Set(), dragFrom: -1, baseRev: 0,
      draftDir: 'default', pendingDraft: null
    }
  };

  var draftTimer = null;
  var draftIdle = null;
  var draftGeneration = 0;
  var lastDraftHtml = '';

  function clone(value) {
    return value == null ? value : JSON.parse(JSON.stringify(value));
  }
  function lsGet(key) { try { return localStorage.getItem(key); } catch (e) { return null; } }
  function lsSet(key, value) { try { localStorage.setItem(key, value); } catch (e) {} }
  function lsDel(key) { try { localStorage.removeItem(key); } catch (e) {} }
  function notify(message, kind, ms) {
    if (K.toast) K.toast(message, kind, ms);
  }

  K.clone = clone;
  K.lsGet = lsGet;
  K.lsSet = lsSet;
  K.lsDel = lsDel;
  K.isDirty = function () { return !!store.model && store.rev !== store.savedRev; };

  function snapshot() {
    return {
      model: clone(store.model),
      baseHtml: store.baseHtml,
      rawMode: store.rawMode
    };
  }

  function resetHistory() {
    clearTimeout(store.history.timer);
    store.history.timer = null;
    store.history.items = store.model ? [snapshot()] : [];
    store.history.index = store.history.items.length - 1;
  }

  function flushHistory() {
    clearTimeout(store.history.timer);
    store.history.timer = null;
    if (!store.model) return;
    var next = snapshot();
    var current = store.history.items[store.history.index];
    if (current && JSON.stringify(current) === JSON.stringify(next)) return;
    store.history.items = store.history.items.slice(0, store.history.index + 1);
    store.history.items.push(next);
    if (store.history.items.length > 60) store.history.items.shift();
    store.history.index = store.history.items.length - 1;
  }

  function queueHistory() {
    clearTimeout(store.history.timer);
    store.history.timer = setTimeout(flushHistory, 450);
  }

  function splitLines(value) {
    return String(value).split('\n').map(function (s) { return s.trim(); }).filter(Boolean);
  }
  function splitTags(value) {
    return String(value).split(/[,，]/).map(function (s) { return s.trim(); }).filter(Boolean);
  }

  function applyAction(action, meta) {
    var model = store.model;
    var p;
    var link;
    switch (action.type) {
      case 'field':
        if (!model) return false;
        if (action.key === 'phrases') model.phrases = splitLines(action.value);
        else if (action.key === 'tags') model.tags = splitTags(action.value);
        else model[action.key] = String(action.value);
        meta.structural = action.key === 'phrases' || action.key === 'github' || action.key === 'gitee';
        meta.patch = { type: 'field', key: action.key };
        return true;
      case 'project':
        p = model && model.projects[action.index];
        if (!p) return false;
        if (action.key === 'tags') p.tags = splitTags(action.value);
        else if (action.key === 'isDev') p.isDev = !!action.value;
        else p[action.key] = String(action.value);
        meta.patch = { type: 'project', index: action.index, key: action.key };
        return true;
      case 'link':
        p = model && model.projects[action.index];
        link = p && p.links[action.linkIndex];
        if (!link) return false;
        link[action.key] = String(action.value);
        meta.patch = { type: 'link', index: action.index, linkIndex: action.linkIndex, key: action.key };
        return true;
      case 'project-add':
        if (!model) return false;
        model.projects.push({
          icon: 'NEW', title: '新项目', desc: '一句话介绍这个项目。', status: '开发中', isDev: true,
          tags: ['Rust', 'AI 辅助'], links: [{ text: 'Gitee', url: '' }]
        });
        store.ui.collapsed.clear();
        meta.structural = true;
        meta.editorRefresh = 'projects';
        return true;
      case 'project-delete':
        if (!model || !model.projects[action.index]) return false;
        model.projects.splice(action.index, 1);
        store.ui.collapsed.clear();
        meta.structural = true;
        meta.editorRefresh = 'projects';
        return true;
      case 'project-move':
        if (!model || action.from < 0 || action.to < 0 || action.from >= model.projects.length || action.to >= model.projects.length || action.from === action.to) return false;
        p = model.projects.splice(action.from, 1)[0];
        model.projects.splice(action.to, 0, p);
        store.ui.collapsed.clear();
        meta.structural = true;
        meta.editorRefresh = 'projects';
        return true;
      case 'link-add':
        p = model && model.projects[action.index];
        if (!p) return false;
        p.links.push({ text: '链接', url: '' });
        meta.patch = { type: 'links', index: action.index };
        meta.editorRefresh = 'projects';
        return true;
      case 'link-delete':
        p = model && model.projects[action.index];
        if (!p || !p.links[action.linkIndex]) return false;
        p.links.splice(action.linkIndex, 1);
        meta.patch = { type: 'links', index: action.index };
        meta.editorRefresh = 'projects';
        return true;
      case 'avatar':
        if (!model || !action.file) return false;
        model.avatar = action.file;
        store.baseHtml = store.baseHtml.replace(/avatar\.(png|jpe?g|webp|gif|avif)/g, action.file);
        meta.structural = true;
        meta.baseChanged = true;
        meta.editorRefresh = 'avatar';
        return true;
      case 'raw':
        if (!model) return false;
        store.baseHtml = String(action.html);
        store.model = K.render.parseSite(store.baseHtml);
        store.rawMode = true;
        meta.structural = true;
        meta.baseChanged = true;
        return true;
      case 'replace':
        store.baseHtml = String(action.html || '');
        store.model = store.baseHtml ? K.render.parseSite(store.baseHtml) : null;
        store.rawMode = false;
        store.ui.collapsed.clear();
        meta.structural = true;
        meta.baseChanged = true;
        meta.immediate = true;
        meta.editorRefresh = 'all';
        meta.clean = !!action.clean;
        meta.resetHistory = true;
        return true;
      case 'history':
        if (!action.snapshot || !action.snapshot.model) return false;
        store.baseHtml = action.snapshot.baseHtml;
        store.model = clone(action.snapshot.model);
        store.rawMode = !!action.snapshot.rawMode;
        store.ui.collapsed.clear();
        meta.structural = true;
        meta.baseChanged = true;
        meta.immediate = true;
        meta.editorRefresh = 'all';
        meta.skipHistory = true;
        return true;
      default:
        return false;
    }
  }

  K.dispatch = function (action) {
    if (!action) return false;
    var previousRev = store.rev;
    var meta = { structural: false, baseChanged: false, immediate: false, editorRefresh: '', clean: false };
    if (!applyAction(action, meta)) return false;

    store.rev++;
    if (meta.baseChanged) store.ui.baseRev++;
    if (meta.clean) store.savedRev = store.rev;

    if (meta.resetHistory) resetHistory();
    else if (!meta.skipHistory) queueHistory();

    if (meta.editorRefresh === 'all' && K.render.refreshEditor) K.render.refreshEditor();
    else if (meta.editorRefresh === 'projects' && K.render.renderProjects) K.render.renderProjects();
    else if (meta.editorRefresh === 'avatar' && K.render.syncAvatar) K.render.syncAvatar();

    if (K.preview) {
      if (meta.structural) K.preview.scheduleFull({ immediate: meta.immediate, force: meta.immediate });
      else K.preview.schedulePatch(meta.patch, previousRev);
    }
    if (!meta.clean) K.schedulePersistence();
    if (K.onStoreChange) K.onStoreChange(action, meta);
    return true;
  };

  K.undo = function () {
    flushHistory();
    if (store.history.index <= 0) { notify('没有可撤销的修改', 'err'); return false; }
    store.history.index--;
    var result = K.dispatch({ type: 'history', snapshot: store.history.items[store.history.index] });
    if (result) notify('已撤销');
    return result;
  };
  K.redo = function () {
    flushHistory();
    if (store.history.index < 0 || store.history.index >= store.history.items.length - 1) { notify('没有可重做的修改', 'err'); return false; }
    store.history.index++;
    var result = K.dispatch({ type: 'history', snapshot: store.history.items[store.history.index] });
    if (result) notify('已重做');
    return result;
  };
  K.resetHistory = resetHistory;
  K.flushHistory = flushHistory;

  K.commitSaved = function (html, savedRev, wasRaw) {
    if (savedRev > store.savedRev) store.savedRev = savedRev;
    if (store.rev !== savedRev) {
      if (K.isDirty()) K.schedulePersistence();
      return store.savedRev === store.rev;
    }
    store.baseHtml = html;
    store.rawMode = false;
    if (wasRaw) {
      store.model = K.render.parseSite(html);
      if (K.render.refreshEditor) K.render.refreshEditor();
    }
    resetHistory();
    K.clearDraft();
    return true;
  };

  function draftKey() { return 'kiopen-draft:' + store.ui.draftDir; }
  function cancelDraftWork() {
    clearTimeout(draftTimer);
    draftTimer = null;
    if (draftIdle != null) {
      if (window.cancelIdleCallback) window.cancelIdleCallback(draftIdle);
      else clearTimeout(draftIdle);
      draftIdle = null;
    }
  }

  K.schedulePersistence = function () {
    if (!store.model) return;
    cancelDraftWork();
    var generation = ++draftGeneration;
    draftTimer = setTimeout(function () {
      draftTimer = null;
      var rev = store.rev;
      var html;
      try { html = K.render.sourceHtml(); } catch (e) { return; }
      if (!html || html === lastDraftHtml) return;
      var write = function () {
        draftIdle = null;
        if (generation !== draftGeneration) return;
        if (rev !== store.rev) { K.schedulePersistence(); return; }
        if (html === lastDraftHtml) return;
        lastDraftHtml = html;
        lsSet(draftKey(), JSON.stringify({ ts: Date.now(), html: html, tab: store.ui.tab, rev: rev }));
      };
      if (window.requestIdleCallback) draftIdle = window.requestIdleCallback(write, { timeout: 1500 });
      else draftIdle = setTimeout(write, 0);
    }, 800);
  };

  K.persistNow = function () {
    if (!store.model || !K.isDirty()) return;
    cancelDraftWork();
    draftGeneration++;
    try {
      var html = K.render.sourceHtml();
      if (!html) return;
      lastDraftHtml = html;
      lsSet(draftKey(), JSON.stringify({ ts: Date.now(), html: html, tab: store.ui.tab, rev: store.rev }));
    } catch (e) {}
  };

  K.clearDraft = function () {
    cancelDraftWork();
    draftGeneration++;
    lsDel(draftKey());
    store.ui.pendingDraft = null;
    lastDraftHtml = '';
    var banner = document.getElementById('draftBanner');
    if (banner) banner.classList.remove('show');
  };

  K.setDraftDir = function (dir) {
    if (!dir || store.ui.draftDir === dir) return;
    cancelDraftWork();
    draftGeneration++;
    store.ui.draftDir = dir;
    store.ui.pendingDraft = null;
    lastDraftHtml = '';
  };

  K.checkDraft = function () {
    var raw = lsGet(draftKey());
    if (!raw) return;
    try {
      var draft = JSON.parse(raw);
      var current = K.render.sourceHtml();
      if (!draft || !draft.html || draft.html === store.baseHtml || draft.html === current) {
        lsDel(draftKey());
        return;
      }
      store.ui.pendingDraft = draft;
      document.getElementById('draftTime').textContent = K.fmtTime(draft.ts);
      document.getElementById('draftBanner').classList.add('show');
    } catch (e) { lsDel(draftKey()); }
  };

  K.restoreDraft = function () {
    var draft = store.ui.pendingDraft;
    if (!draft) return;
    store.ui.pendingDraft = null;
    document.getElementById('draftBanner').classList.remove('show');
    K.dispatch({ type: 'replace', html: draft.html });
    notify('草稿已恢复，记得保存', 'ok');
  };

  K.discardDraft = function () {
    K.clearDraft();
    notify('已丢弃本地草稿', 'ok');
  };

  K.fmtTime = function (ms) {
    if (!ms) return '未知时间';
    var d = new Date(Number(ms));
    function pad(n) { return n < 10 ? '0' + n : '' + n; }
    return (d.getMonth() + 1) + '-' + pad(d.getDate()) + ' ' + pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds());
  };
})(K);
