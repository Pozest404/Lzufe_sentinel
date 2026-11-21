var K = window.K || (window.K = {});

(function (K) {
  'use strict';

  var $ = K.$;
  var esc = K.esc;
  var attr = K.attr;
  var store = K.store;
  var paletteItems = [];
  var paletteIndex = 0;
  var setupTimer = null;
  var publishTimer = null;
  var publishPolling = false;
  var publishAuto = false;

  function toast(message, kind, ms) {
    var element = document.createElement('div');
    element.className = 'toast' + (kind ? ' ' + kind : '');
    var span = document.createElement('span');
    span.textContent = message;
    element.appendChild(span);
    var close = document.createElement('button');
    close.type = 'button';
    close.className = 'x';
    close.textContent = '✕';
    close.setAttribute('aria-label', '关闭提示');
    close.onclick = function () { element.remove(); };
    element.appendChild(close);
    $('#toasts').appendChild(element);
    setTimeout(function () { element.remove(); }, ms || 3600);
  }
  K.toast = toast;

  function setStatus(text, kind) {
    var suffix = kind === 'ok' || !kind ? '' : ' ' + kind;
    $('#statusText').textContent = text;
    $('#statusDot').className = 'dot' + suffix;
    $('#topStateText').textContent = text;
    $('#topState').className = 'state' + suffix;
  }
  function setConn(ok) {
    $('#connPill').classList.toggle('off', !ok);
    $('#connText').textContent = ok ? '已连接' : '连接断开';
  }
  function showModal(title, text) {
    $('#modalTitle').textContent = title;
    $('#modalBody').textContent = text;
    $('#modal').classList.add('show');
  }

  K.onStoreChange = function (action, meta) {
    if (meta.clean) setStatus('已加载', 'ok');
    else if (action.type === 'avatar') setStatus('头像已更新', 'dirty');
    else setStatus('未保存', 'dirty');
    K.render.updateCounters();
    if (store.ui.tab === 'raw') K.render.updateRawCount();
  };

  function applyTheme(theme) {
    store.ui.theme = theme === 'light' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', store.ui.theme);
    $('#themeBtn').textContent = store.ui.theme === 'light' ? '☾' : '◐';
    $('#themeBtn').title = store.ui.theme === 'light' ? '切换到深色主题' : '切换到浅色主题';
    K.lsSet('kiopen-theme', store.ui.theme);
  }
  function toggleTheme() { applyTheme(store.ui.theme === 'light' ? 'dark' : 'light'); }
  function setAutoPublish(on, silent) {
    store.ui.autoPublish = !!on;
    $('#autoPublishBtn').setAttribute('aria-pressed', store.ui.autoPublish ? 'true' : 'false');
    K.lsSet('kiopen-autopublish', store.ui.autoPublish ? '1' : '0');
    if (!silent) toast(store.ui.autoPublish ? '已开启：保存后自动发布上线' : '已关闭自动发布', 'ok');
  }
  function togglePreview() {
    var hidden = $('.workspace').classList.toggle('hide-preview');
    if (!hidden) K.preview.flush();
    toast(hidden ? '已隐藏预览区' : '已显示预览区');
  }

  function buildCommands() {
    return [
      { name: '保存', key: 'Ctrl S', run: function () { save(); } },
      { name: '发布上线', key: 'Ctrl Enter', run: function () { publish(); } },
      { name: '撤销上一步', key: 'Ctrl Z', run: K.undo },
      { name: '重做', key: 'Ctrl Shift Z', run: K.redo },
      { name: '切换到：常用', run: function () { switchTab('basic'); } },
      { name: '切换到：项目', run: function () { switchTab('projects'); } },
      { name: '切换到：头像', run: function () { switchTab('avatar'); } },
      { name: '切换到：SEO', run: function () { switchTab('seo'); } },
      { name: '切换到：历史备份', run: function () { switchTab('history'); } },
      { name: '切换到：源码', run: function () { switchTab('raw'); } },
      { name: store.ui.theme === 'light' ? '切换到深色主题' : '切换到浅色主题', run: toggleTheme },
      { name: '刷新预览', run: K.preview.refresh },
      { name: '显示 / 隐藏预览区', run: togglePreview },
      { name: '开启 / 关闭自动发布', run: function () { setAutoPublish(!store.ui.autoPublish); } },
      { name: '打开线上站点', run: function () { window.open($('#siteLink').href, '_blank'); } },
      { name: '重新加载主页数据', run: function () { load(); } },
      { name: '查看快捷键帮助', key: '?', run: showHelp },
      { name: '新建项目', run: function () { switchTab('projects'); addProject(); } }
    ];
  }
  function openPalette() {
    $('#palette').classList.add('show');
    $('#paletteInput').value = '';
    paletteIndex = 0;
    renderPalette('');
    setTimeout(function () { $('#paletteInput').focus(); }, 0);
  }
  function closePalette() { $('#palette').classList.remove('show'); }
  function renderPalette(query) {
    var all = buildCommands();
    var q = String(query || '').trim().toLowerCase();
    paletteItems = q ? all.filter(function (command) { return command.name.toLowerCase().indexOf(q) >= 0; }) : all;
    if (!paletteItems.length) {
      $('#paletteList').innerHTML = '<div class="palette-empty">没有匹配的命令</div>';
      return;
    }
    if (paletteIndex >= paletteItems.length) paletteIndex = 0;
    $('#paletteList').innerHTML = paletteItems.map(function (command, index) {
      return '<div class="cmd' + (index === paletteIndex ? ' active' : '') + '" data-i="' + index + '">' +
        '<span>' + esc(command.name) + '</span><span class="grow"></span>' +
        (command.key ? '<span class="k">' + esc(command.key) + '</span>' : '') + '</div>';
    }).join('');
  }
  function runPalette(index) {
    var command = paletteItems[index];
    if (!command) return;
    closePalette();
    command.run();
  }

  function showHelp() {
    showModal('快捷键与技巧', [
      'Ctrl + K          打开命令面板',
      'Ctrl + S          保存到本地（自动备份）',
      'Ctrl + Enter      保存并发布上线',
      'Ctrl + Z          撤销表单修改（输入框内为系统撤销）',
      'Ctrl + Shift + Z  重做',
      'Esc               关闭弹窗 / 命令面板',
      '',
      '· 编辑时草稿会自动保存在浏览器里，刷新或误关页面可恢复',
      '· 「历史」页可预览或恢复任意一次自动备份',
      '· 开启「自动发布」后，每次保存成功会自动提交并推送',
      '· 项目卡片左侧 ⠿ 手柄可拖动排序，也可用 ↑ ↓ 按钮',
      '· 预览区支持桌面 / 手机宽度，可隐藏以扩大编辑区'
    ].join('\n'));
  }

  async function loadBackups() {
    var box = $('#backupList');
    box.innerHTML = '<div class="bk-empty">加载中…</div>';
    try {
      var response = await fetch('/api/backups', { cache: 'no-store' });
      var json = await response.json();
      if (!response.ok || !json.ok) throw new Error(json.error || ('HTTP ' + response.status));
      if (!json.items.length) {
        box.innerHTML = '<div class="bk-empty">还没有备份，保存一次就会生成。</div>';
        return;
      }
      box.innerHTML = json.items.map(function (item) {
        return '<div class="bk-item" data-name="' + attr(item.name) + '">' +
          '<div class="bk-info"><div class="bk-name">' + esc(item.name) + '</div>' +
          '<div class="bk-meta">' + K.fmtTime(item.time) + ' · ' + (item.bytes / 1024).toFixed(1) + ' KB</div></div>' +
          '<button class="mini bk-view" type="button">预览</button>' +
          '<button class="mini bk-restore" type="button">恢复</button>' +
          '</div>';
      }).join('');
    } catch (e) {
      box.innerHTML = '<div class="bk-empty">加载失败：' + esc(String(e.message || e)) + '</div>';
    }
  }

  async function fetchBackup(name) {
    return fetch('/api/backup/' + encodeURIComponent(name), { cache: 'no-store' });
  }
  async function viewBackup(name) {
    try {
      var response = await fetchBackup(name);
      if (!response.ok) throw new Error('HTTP ' + response.status);
      K.preview.showExternal(await response.text());
      toast('正在预览备份 ' + name, 'ok');
    } catch (e) { toast('预览失败：' + String(e.message || e), 'err'); }
  }

  async function restoreRequest(name) {
    var response = await fetch('/api/backups/restore', {
      method: 'POST',
      headers: { 'Content-Type': 'text/plain; charset=utf-8' },
      body: name
    });
    if (response.status === 404 || response.status === 405) {
      response = await fetch('/api/restore/' + encodeURIComponent(name), { method: 'POST' });
    }
    return response;
  }
  async function restoreBackup(name) {
    if (!confirm('恢复 ' + name + '？当前版本会先自动备份。')) return;
    try {
      var response = await restoreRequest(name);
      var json = await response.json();
      if (!response.ok || !json.ok) throw new Error(json.error || ('HTTP ' + response.status));
      await load();
      await loadBackups();
      toast('已恢复到 ' + name, 'ok');
    } catch (e) { toast('恢复失败：' + String(e.message || e), 'err', 6000); }
  }

  function switchTab(name) {
    var wasRaw = store.ui.tab === 'raw';
    if (wasRaw && name !== 'raw') store.rawMode = false;
    store.ui.tab = name;
    Array.prototype.slice.call(document.querySelectorAll('.nav-item')).forEach(function (button) {
      var on = button.getAttribute('data-tab') === name;
      button.classList.toggle('active', on);
      if (on) button.setAttribute('aria-current', 'page');
      else button.removeAttribute('aria-current');
    });
    Array.prototype.slice.call(document.querySelectorAll('.pane')).forEach(function (pane) {
      pane.classList.toggle('active', pane.id === 'pane-' + name);
    });
    if (wasRaw && name !== 'raw') K.render.refreshEditor();
    if (name === 'raw') {
      store.rawMode = false;
      $('#rawHtml').value = K.render.sourceHtml();
      K.render.updateRawCount();
    }
    if (name === 'seo') K.render.updateCounters();
    if (name === 'history') loadBackups();
  }

  function showSetup(info) {
    info = info || {};
    var running = info.setup_running === true;
    var error = info.setup_error || '';
    $('#setup').classList.add('show');
    $('#setupTitle').textContent = running ? '正在准备主页仓库…' : '主页仓库未就绪';
    $('#setupMsg').textContent = running ? '首次运行需要克隆仓库，请稍候（最多约 2 分钟）。' : '克隆或更新失败，请检查网络后重试。';
    $('#setupError').textContent = error;
    $('#setupError').classList.toggle('show', !!error);
    $('#setupRetry').disabled = running;
    clearTimeout(setupTimer);
    if (running) setupTimer = setTimeout(pollSetup, 2000);
  }
  function pollSetup() {
    if (!$('#setup').classList.contains('show')) return;
    fetch('/api/info', { cache: 'no-store' }).then(function (response) { return response.json(); }).then(function (info) {
      if (info.ready) {
        clearTimeout(setupTimer);
        $('#setup').classList.remove('show');
        toast('仓库已就绪', 'ok');
        load();
      } else showSetup(info);
    }).catch(function () { setupTimer = setTimeout(pollSetup, 3000); });
  }
  async function retrySetup() {
    $('#setupRetry').disabled = true;
    $('#setupError').classList.remove('show');
    try {
      var response = await fetch('/api/setup-retry', { method: 'POST' });
      if (response.status === 404) await fetch('/api/setup', { method: 'POST' });
    } catch (e) {}
    showSetup({ setup_running: true });
  }

  async function load() {
    try {
      var responses = await Promise.all([
        fetch('/api/site', { cache: 'no-store' }),
        fetch('/api/info', { cache: 'no-store' })
      ]);
      var info = responses[1].ok ? await responses[1].json() : {};
      if (info.site_url) $('#siteLink').href = info.site_url;
      if (info.dir) {
        $('#dirPath').textContent = info.dir;
        K.setDraftDir(info.dir);
      }
      if (!responses[0].ok || info.ready === false) {
        K.dispatch({ type: 'replace', html: '', clean: true });
        setConn(!!responses[1].ok);
        setStatus(info.setup_running ? '初始化中…' : '等待初始化', info.setup_running ? 'busy' : 'err');
        showSetup(info);
        return;
      }
      var html = await responses[0].text();
      K.dispatch({ type: 'replace', html: html, clean: true });
      setConn(true);
      setStatus('已加载', 'ok');
      K.checkDraft();
    } catch (e) {
      setConn(false);
      setStatus('加载失败', 'err');
      showModal('无法加载主页', String(e && e.message ? e.message : e));
    }
  }

  async function doSave() {
    if (!store.model) return false;
    var html;
    try { html = K.render.sourceHtml(); }
    catch (e) { toast(String(e.message || e), 'err'); return false; }
    var rawActive = store.rawMode || store.ui.tab === 'raw';
    if (rawActive && !/<html/i.test(store.baseHtml)) {
      toast('内容不完整，缺少 <html>', 'err');
      return false;
    }
    if (!rawActive && !store.model.name.trim()) { toast('姓名不能为空', 'err'); return false; }
    if (!/<html/i.test(html)) { toast('内容不完整，缺少 <html>', 'err'); return false; }
    var savedRev = store.rev;
    var wasRaw = store.rawMode;
    setStatus('保存中…', 'busy');
    try {
      var response = await fetch('/api/save', {
        method: 'POST',
        headers: { 'Content-Type': 'text/html; charset=utf-8' },
        body: html
      });
      var json = await response.json();
      if (!response.ok || !json.ok) throw new Error(json.error || ('HTTP ' + response.status));
      var clean = K.commitSaved(html, savedRev, wasRaw);
      setStatus(clean ? '已保存' : '未保存', clean ? 'ok' : 'dirty');
      toast('已保存' + (json.backup ? '，已生成备份' : ''), 'ok');
      return true;
    } catch (e) {
      setStatus('保存失败', 'err');
      toast(String(e && e.message ? e.message : e), 'err', 6000);
      return false;
    }
  }

  async function save() {
    var ok = await doSave();
    if (ok && store.ui.autoPublish) await enqueuePublish(true);
    return ok;
  }

  function logTail(log) {
    var text = String(log || '').trim();
    if (!text) return '发布失败，但没有返回日志。';
    var lines = text.split('\n');
    text = lines.slice(-30).join('\n');
    return text.length > 6000 ? text.slice(-6000) : text;
  }
  function publishError(message, auto) {
    var tail = logTail(message);
    setStatus('发布失败', 'err');
    if (auto) toast('自动发布失败：' + tail, 'err', 7000);
    showModal('发布失败', tail);
  }
  async function pollPublishStatus() {
    try {
      var response = await fetch('/api/publish/status', { cache: 'no-store' });
      var json = await response.json();
      if (!response.ok) throw new Error(json.error || ('HTTP ' + response.status));
      if (json.state === 'queued' || json.state === 'running') {
        setStatus(json.state === 'queued' ? '发布排队中…' : '发布中…', 'busy');
        publishTimer = setTimeout(function () { pollPublishStatus(); }, 1000);
        return;
      }
      publishPolling = false;
      publishTimer = null;
      if (json.state === 'success') {
        var unchanged = json.changed === false;
        var message = unchanged ? '没有需要发布的改动' : (publishAuto ? '已自动发布，约 1 分钟后上线' : '已发布，约 1 分钟后上线');
        setStatus(K.isDirty() ? '未保存' : '已发布', K.isDirty() ? 'dirty' : 'ok');
        toast(message, 'ok');
        if (!publishAuto) showModal('发布日志', json.log || '完成');
      } else if (json.state === 'error') {
        publishError(json.log || json.error, publishAuto);
      } else {
        setStatus(K.isDirty() ? '未保存' : '已加载', K.isDirty() ? 'dirty' : 'ok');
      }
      publishAuto = false;
    } catch (e) {
      publishPolling = false;
      publishTimer = null;
      var auto = publishAuto;
      publishAuto = false;
      publishError(String(e && e.message ? e.message : e), auto);
    }
  }

  async function enqueuePublish(auto) {
    try {
      var response = await fetch('/api/publish', { method: 'POST' });
      var json = await response.json();
      if (!response.ok || !json.ok) throw new Error(json.error || ('HTTP ' + response.status));
      publishAuto = publishPolling ? publishAuto && auto : auto;
      setStatus('发布排队中…', 'busy');
      clearTimeout(publishTimer);
      publishPolling = true;
      publishTimer = setTimeout(function () { pollPublishStatus(); }, 1000);
      return true;
    } catch (e) {
      publishError(String(e && e.message ? e.message : e), auto);
      return false;
    }
  }

  async function publish(options) {
    options = options || {};
    if (options.auto) { await enqueuePublish(true); return; }
    if (K.isDirty()) {
      if (!confirm('还有未保存的修改，先保存再发布吗？')) return;
      var ok = await doSave();
      if (!ok) return;
    }
    if (!confirm('发布到 GitHub？约 1 分钟后上线。')) return;
    await enqueuePublish(false);
  }

  async function uploadAvatar(file) {
    if (!file) return;
    if (file.type === 'image/heic' || /\.heic$/i.test(file.name || '')) {
      toast('HEIC 暂不支持，请在相册里导出为 JPG / PNG 后再上传', 'err', 7000);
      return;
    }
    if (!/^image\/(png|jpeg|webp|gif|avif)$/.test(file.type)) { toast('仅支持 PNG / JPG / WEBP / GIF / AVIF', 'err'); return; }
    if (file.size > 10 * 1024 * 1024) { toast('图片不能超过 10MB', 'err'); return; }
    $('#avatarImg').src = URL.createObjectURL(file);
    setStatus('上传头像…', 'busy');
    try {
      var response = await fetch('/api/avatar', {
        method: 'POST',
        headers: { 'Content-Type': file.type },
        body: file
      });
      var json = await response.json();
      if (!response.ok || !json.ok) throw new Error(json.error || ('HTTP ' + response.status));
      K.dispatch({ type: 'avatar', file: json.file });
      $('#avatarName').textContent = json.file + ' · ' + (json.bytes / 1024).toFixed(1) + ' KB';
      toast('头像已写入仓库，发布后线上生效', 'ok');
    } catch (e) {
      setStatus('头像上传失败', 'err');
      var message = String(e && e.message ? e.message : e);
      if (e instanceof TypeError) message = '无法连接本地服务，请确认 kiopen-homepage 仍在运行';
      toast(message, 'err', 7000);
    }
  }

  Array.prototype.slice.call(document.querySelectorAll('.nav-item')).forEach(function (button) {
    button.addEventListener('click', function () { switchTab(button.getAttribute('data-tab')); });
  });

  var basicMap = { fName: 'name', fDesc: 'desc', fPhrases: 'phrases', fTags: 'tags', fGithub: 'github', fGitee: 'gitee', fEmail: 'email' };
  Object.keys(basicMap).forEach(function (id) {
    $('#' + id).addEventListener('input', function (event) {
      K.dispatch({ type: 'field', key: basicMap[id], value: event.target.value });
    });
  });
  ['seoTitle', 'seoDesc'].forEach(function (id) {
    $('#' + id).addEventListener('input', function (event) {
      K.dispatch({ type: 'field', key: id, value: event.target.value });
    });
  });

  $('#projectList').addEventListener('input', function (event) {
    var card = event.target.closest('.pcard');
    if (!card) return;
    var index = Number(card.getAttribute('data-i'));
    var field = event.target.getAttribute('data-f');
    if (field) {
      K.dispatch({ type: 'project', index: index, key: field, value: event.target.value });
      return;
    }
    var row = event.target.closest('.link-row');
    if (row) {
      K.dispatch({
        type: 'link', index: index, linkIndex: Number(row.getAttribute('data-k')),
        key: event.target.classList.contains('l-text') ? 'text' : 'url', value: event.target.value
      });
    }
  });
  $('#projectList').addEventListener('change', function (event) {
    if (!event.target.classList.contains('p-dev')) return;
    var card = event.target.closest('.pcard');
    K.dispatch({ type: 'project', index: Number(card.getAttribute('data-i')), key: 'isDev', value: event.target.checked });
  });
  $('#projectList').addEventListener('click', function (event) {
    var card = event.target.closest('.pcard');
    if (!card) return;
    var index = Number(card.getAttribute('data-i'));
    if (event.target.closest('.p-del')) {
      if (confirm('删除这个项目？')) K.dispatch({ type: 'project-delete', index: index });
    } else if (event.target.closest('.p-up')) {
      moveProject(index, -1);
    } else if (event.target.closest('.p-down')) {
      moveProject(index, 1);
    } else if (event.target.closest('.p-collapse')) {
      if (store.ui.collapsed.has(index)) store.ui.collapsed.delete(index);
      else store.ui.collapsed.add(index);
      K.render.renderProjects();
    } else if (event.target.closest('.p-add-link')) {
      K.dispatch({ type: 'link-add', index: index });
    } else if (event.target.closest('.p-del-link')) {
      K.dispatch({ type: 'link-delete', index: index, linkIndex: Number(event.target.closest('.link-row').getAttribute('data-k')) });
    }
  });

  function addProject() { K.dispatch({ type: 'project-add' }); }
  function moveProject(index, delta) { K.dispatch({ type: 'project-move', from: index, to: index + delta }); }
  $('#addProject').addEventListener('click', addProject);

  $('#projectList').addEventListener('dragstart', function (event) {
    var grip = event.target.closest('.p-grip');
    if (!grip) { event.preventDefault(); return; }
    var card = grip.closest('.pcard');
    store.ui.dragFrom = Number(card.getAttribute('data-i'));
    card.classList.add('dragging');
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData('text/plain', String(store.ui.dragFrom));
    }
  });
  $('#projectList').addEventListener('dragover', function (event) {
    if (store.ui.dragFrom < 0) return;
    event.preventDefault();
    var card = event.target.closest('.pcard');
    if (!card) return;
    Array.prototype.slice.call(document.querySelectorAll('.pcard')).forEach(function (item) {
      item.classList.toggle('drop-target', item === card && Number(item.getAttribute('data-i')) !== store.ui.dragFrom);
    });
  });
  $('#projectList').addEventListener('drop', function (event) {
    if (store.ui.dragFrom < 0) return;
    event.preventDefault();
    var card = event.target.closest('.pcard');
    if (!card) return;
    var to = Number(card.getAttribute('data-i'));
    var from = store.ui.dragFrom;
    store.ui.dragFrom = -1;
    if (from === to || isNaN(to)) { K.render.renderProjects(); return; }
    K.dispatch({ type: 'project-move', from: from, to: to });
  });
  $('#projectList').addEventListener('dragend', function () {
    store.ui.dragFrom = -1;
    Array.prototype.slice.call(document.querySelectorAll('.pcard')).forEach(function (card) {
      card.classList.remove('dragging', 'drop-target');
    });
  });

  $('#rawHtml').addEventListener('input', function (event) {
    K.dispatch({ type: 'raw', html: event.target.value });
  });
  $('#avatarInput').addEventListener('change', function (event) {
    if (event.target.files && event.target.files[0]) uploadAvatar(event.target.files[0]);
    event.target.value = '';
  });
  $('#dropzone').addEventListener('click', function () { $('#avatarInput').click(); });
  ['dragenter', 'dragover'].forEach(function (name) {
    $('#dropzone').addEventListener(name, function (event) { event.preventDefault(); $('#dropzone').classList.add('drag'); });
  });
  ['dragleave', 'drop'].forEach(function (name) {
    $('#dropzone').addEventListener(name, function (event) { event.preventDefault(); $('#dropzone').classList.remove('drag'); });
  });
  $('#dropzone').addEventListener('drop', function (event) {
    if (event.dataTransfer && event.dataTransfer.files[0]) uploadAvatar(event.dataTransfer.files[0]);
  });

  $('#saveBtn').addEventListener('click', save);
  Array.prototype.slice.call(document.querySelectorAll('.btn-save-2, .btn-save-3, .btn-save-4')).forEach(function (button) {
    button.addEventListener('click', save);
  });
  $('#publishBtn').addEventListener('click', function () { publish(); });
  $('#publishBtn2').addEventListener('click', function () { publish(); });

  $('#deviceBtn').addEventListener('click', function () {
    var phone = $('#previewBody').classList.toggle('phone');
    $('#deviceBtn').textContent = phone ? '桌面' : '手机';
  });
  $('#refreshBtn').addEventListener('click', K.preview.refresh);
  $('#hidePreviewBtn').addEventListener('click', togglePreview);
  $('#previewToggle').addEventListener('click', togglePreview);
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) K.preview.flush();
  });

  $('#themeBtn').addEventListener('click', toggleTheme);
  $('#helpBtn').addEventListener('click', showHelp);
  $('#autoPublishBtn').addEventListener('click', function () { setAutoPublish(!store.ui.autoPublish); });
  $('#draftRestore').addEventListener('click', function () {
    K.restoreDraft();
    setStatus('已恢复草稿', 'dirty');
  });
  $('#draftDiscard').addEventListener('click', K.discardDraft);
  $('#setupRetry').addEventListener('click', retrySetup);
  $('#setupClose').addEventListener('click', function () {
    clearTimeout(setupTimer);
    $('#setup').classList.remove('show');
  });

  $('#backupList').addEventListener('click', function (event) {
    var item = event.target.closest('.bk-item');
    if (!item) return;
    var name = item.getAttribute('data-name');
    if (event.target.closest('.bk-view')) viewBackup(name);
    else if (event.target.closest('.bk-restore')) restoreBackup(name);
  });
  $('#refreshBackups').addEventListener('click', loadBackups);

  $('#paletteInput').addEventListener('input', function (event) {
    paletteIndex = 0;
    renderPalette(event.target.value);
  });
  $('#paletteInput').addEventListener('keydown', function (event) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (paletteItems.length) { paletteIndex = (paletteIndex + 1) % paletteItems.length; renderPalette($('#paletteInput').value); }
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (paletteItems.length) { paletteIndex = (paletteIndex - 1 + paletteItems.length) % paletteItems.length; renderPalette($('#paletteInput').value); }
    } else if (event.key === 'Enter') {
      event.preventDefault();
      runPalette(paletteIndex);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      closePalette();
    }
  });
  $('#paletteList').addEventListener('click', function (event) {
    var item = event.target.closest('.cmd');
    if (item) runPalette(Number(item.getAttribute('data-i')));
  });
  $('#palette').addEventListener('click', function (event) { if (event.target === $('#palette')) closePalette(); });

  function fallbackCopy(text) {
    var textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);
    textarea.select();
    try { document.execCommand('copy'); } catch (e) {}
    textarea.remove();
  }
  $('#modalCopy').addEventListener('click', function () {
    var text = $('#modalBody').textContent || '';
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(function () { toast('已复制到剪贴板', 'ok'); }, function () {
        fallbackCopy(text);
        toast('已复制到剪贴板', 'ok');
      });
    } else {
      fallbackCopy(text);
      toast('已复制到剪贴板', 'ok');
    }
  });
  $('#modalClose').addEventListener('click', function () { $('#modal').classList.remove('show'); });
  $('#modal').addEventListener('click', function (event) { if (event.target === $('#modal')) $('#modal').classList.remove('show'); });

  document.addEventListener('keydown', function (event) {
    var mod = event.ctrlKey || event.metaKey;
    var key = event.key.toLowerCase();
    var tag = (event.target.tagName || '').toLowerCase();
    var editing = tag === 'input' || tag === 'textarea' || event.target.isContentEditable;
    if (event.key === 'Escape') {
      if ($('#palette').classList.contains('show')) { closePalette(); return; }
      $('#modal').classList.remove('show');
      return;
    }
    if (mod && key === 'k') {
      event.preventDefault();
      if ($('#palette').classList.contains('show')) closePalette();
      else openPalette();
      return;
    }
    if (mod && key === 's') { event.preventDefault(); save(); return; }
    if (mod && event.key === 'Enter') { event.preventDefault(); publish(); return; }
    if (mod && key === 'z' && !editing) {
      event.preventDefault();
      if (event.shiftKey) K.redo();
      else K.undo();
      return;
    }
    if (mod && key === 'y' && !editing) { event.preventDefault(); K.redo(); return; }
    if (event.key === '?' && !editing) { event.preventDefault(); showHelp(); }
  });

  window.addEventListener('beforeunload', function (event) {
    if (K.isDirty()) {
      event.preventDefault();
      event.returnValue = '';
    }
  });
  window.addEventListener('pagehide', K.persistNow);

  applyTheme(K.lsGet('kiopen-theme') || ((window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches) ? 'light' : 'dark'));
  setAutoPublish(K.lsGet('kiopen-autopublish') !== '0', true);
  $('#versionText').textContent = 'v3.0.0';
  load();
})(K);
