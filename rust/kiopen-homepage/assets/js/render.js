var K = window.K || (window.K = {});

(function (K) {
  'use strict';

  var ARROW = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 17 17 7M9 7h8v8"></path></svg>';
  var buildCache = { rev: -1, baseHtml: null, model: null, html: '' };

  function $(selector) { return document.querySelector(selector); }
  function esc(value) {
    return String(value == null ? '' : value).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }
  function attr(value) { return esc(value); }
  function stripProto(url) { return String(url || '').replace(/^https?:\/\//, '').replace(/\/+$/, ''); }
  function userOf(url) {
    try { return new URL(url).pathname.split('/').filter(Boolean)[0] || ''; } catch (e) { return ''; }
  }

  K.$ = $;
  K.esc = esc;
  K.attr = attr;
  K.stripProto = stripProto;
  K.ARROW = ARROW;

  function contactKind(href) {
    if (href.indexOf('github.com') >= 0) return 'gh';
    if (href.indexOf('gitee.com') >= 0) return 'gt';
    if (href.indexOf('mailto:') === 0) return 'mail';
    return '';
  }

  function parseSite(html) {
    var doc = new DOMParser().parseFromString(html, 'text/html');
    var socials = Array.prototype.slice.call(doc.querySelectorAll('.socials a'));
    function hrefOf(predicate) {
      for (var i = 0; i < socials.length; i++) {
        var href = socials[i].getAttribute('href') || '';
        if (predicate(href)) return href;
      }
      return '';
    }
    var match = html.match(/var phrases = \[([\s\S]*?)\];/);
    var phrases = match ? Array.prototype.slice.call(match[1].matchAll(/'([^']*)'|"([^"]*)"/g)).map(function (item) {
      return item[1] != null ? item[1] : item[2];
    }) : [];
    var avatarElement = doc.querySelector('.avatar-core img');
    var avatar = avatarElement ? (avatarElement.getAttribute('src') || 'avatar.png').split('/').pop() : 'avatar.png';
    var projects = Array.prototype.slice.call(doc.querySelectorAll('.project')).map(function (project) {
      var badge = project.querySelector('.badge');
      return {
        icon: (project.querySelector('.project-icon') || {}).textContent || '',
        title: (project.querySelector('h3') || {}).textContent || '',
        desc: (project.querySelector('p') || {}).innerHTML || '',
        status: badge ? badge.textContent.trim() : '',
        isDev: badge ? badge.classList.contains('dev') : false,
        tags: Array.prototype.slice.call(project.querySelectorAll('.project-tags span')).map(function (span) { return span.textContent.trim(); }),
        links: Array.prototype.slice.call(project.querySelectorAll('.project-links a')).map(function (a) {
          var first = a.childNodes[0];
          return { text: ((first && first.textContent) || a.textContent || '').trim(), url: a.getAttribute('href') || '' };
        })
      };
    });
    var description = doc.querySelector('meta[name="description"]');
    return {
      name: ((doc.querySelector('.name') || {}).textContent || '').trim(),
      desc: ((doc.querySelector('.hero-desc') || {}).innerHTML || '').trim(),
      tags: Array.prototype.slice.call(doc.querySelectorAll('.tags span')).map(function (span) { return span.textContent.trim(); }),
      github: hrefOf(function (href) { return href.indexOf('github.com') >= 0; }),
      gitee: hrefOf(function (href) { return href.indexOf('gitee.com') >= 0; }),
      email: hrefOf(function (href) { return href.indexOf('mailto:') === 0; }).replace(/^mailto:/, ''),
      phrases: phrases,
      projects: projects,
      seoTitle: ((doc.querySelector('title') || {}).textContent || '').trim(),
      seoDesc: (description ? description.getAttribute('content') || '' : '').trim(),
      avatar: avatar
    };
  }

  function setMeta(doc, selector, value) {
    var element = doc.querySelector(selector);
    if (element) element.setAttribute('content', value);
  }

  function fillTags(container, tags) {
    container.textContent = '';
    tags.forEach(function (tag) {
      var span = container.ownerDocument.createElement('span');
      span.textContent = tag;
      container.appendChild(span);
    });
  }

  function applyName(doc, model) {
    var element = doc.querySelector('.name');
    if (element) element.textContent = model.name;
    return !!element;
  }
  function applyDescription(doc, model) {
    var element = doc.querySelector('.hero-desc');
    if (element) element.innerHTML = model.desc;
    return !!element;
  }
  function applyTags(doc, model) {
    var element = doc.querySelector('.tags');
    if (element) fillTags(element, model.tags);
    return !!element;
  }
  function applySeo(doc, model) {
    doc.title = model.seoTitle || model.name;
    setMeta(doc, 'meta[name="description"]', model.seoDesc);
    setMeta(doc, 'meta[property="og:title"]', model.seoTitle || model.name);
    setMeta(doc, 'meta[property="og:description"]', model.seoDesc);
    setMeta(doc, 'meta[name="twitter:title"]', model.seoTitle || model.name);
    setMeta(doc, 'meta[name="twitter:description"]', model.seoDesc);
  }
  function applyContacts(doc, model, emailOnly) {
    Array.prototype.slice.call(doc.querySelectorAll('.socials a, .contact-item')).forEach(function (a) {
      var kind = contactKind(a.getAttribute('href') || '');
      if (emailOnly && kind !== 'mail') return;
      var strong = a.querySelector('strong');
      if (kind === 'gh') {
        a.setAttribute('href', model.github);
        if (strong && a.classList.contains('contact-item')) strong.textContent = stripProto(model.github);
      } else if (kind === 'gt') {
        a.setAttribute('href', model.gitee);
        if (strong && a.classList.contains('contact-item')) strong.textContent = stripProto(model.gitee);
      } else if (kind === 'mail') {
        a.setAttribute('href', 'mailto:' + model.email);
        if (strong && a.classList.contains('contact-item')) strong.textContent = model.email;
      }
    });
    if (!emailOnly || doc.getElementById('copyEmail')) {
      var copy = doc.getElementById('copyEmail');
      if (copy) copy.setAttribute('data-email', model.email);
    }
  }
  function applyAvatar(doc, model) {
    var image = doc.querySelector('.avatar-core img');
    if (!image || !model.avatar) return;
    var src = image.getAttribute('src') || 'avatar.png';
    image.setAttribute('src', src.replace(/[^/]*$/, model.avatar));
  }

  function validLinks(project) {
    return project.links.filter(function (link) { return link.url && link.text; });
  }
  function linkLabel(link) { return link.text + ' '; }
  function projectLinksHtml(project) {
    return validLinks(project).map(function (link) {
      return '<a href="' + attr(link.url) + '" target="_blank" rel="noopener">' + esc(link.text) + ' ' + ARROW + '</a>';
    }).join('');
  }
  function projectHtml(project, index) {
    var badge = project.status ? '<span class="badge' + (project.isDev ? ' dev' : '') + '">' + esc(project.status) + '</span>' : '';
    return '<article class="project reveal" style="--d: ' + (index * 0.07).toFixed(2) + 's">' +
      '<div class="project-top"><div class="project-icon" aria-hidden="true">' + esc(project.icon || '•') + '</div>' + badge + '</div>' +
      '<h3>' + esc(project.title) + '</h3>' +
      '<p>' + project.desc + '</p>' +
      '<div class="project-tags">' + project.tags.map(function (tag) { return '<span>' + esc(tag) + '</span>'; }).join('') + '</div>' +
      '<div class="project-links">' + projectLinksHtml(project) + '</div>' +
      '</article>';
  }

  function applyProjects(doc, model) {
    var grid = doc.querySelector('.projects-grid');
    if (grid) grid.innerHTML = model.projects.map(projectHtml).join('');
    var stat = doc.getElementById('statProjects');
    if (stat) stat.setAttribute('data-target', String(model.projects.length));
  }

  function safeMarkup(html) {
    var template = document.createElement('template');
    template.innerHTML = String(html || '');
    if (template.content.querySelector('script, style, iframe')) return false;
    var elements = template.content.querySelectorAll('*');
    for (var i = 0; i < elements.length; i++) {
      for (var j = 0; j < elements[i].attributes.length; j++) {
        if (/^on/i.test(elements[i].attributes[j].name)) return false;
      }
    }
    return true;
  }

  function renderProjectLinks(container, project, force) {
    var links = validLinks(project);
    var anchors = Array.prototype.slice.call(container.querySelectorAll('a'));
    var reusable = !force && anchors.length === links.length;
    if (reusable) {
      for (var i = 0; i < anchors.length; i++) {
        if (!anchors[i].firstChild || anchors[i].firstChild.nodeType !== 3 || !anchors[i].querySelector('svg')) {
          reusable = false;
          break;
        }
      }
    }
    if (!reusable) {
      container.innerHTML = projectLinksHtml(project);
      return;
    }
    anchors.forEach(function (a, index) {
      var link = links[index];
      a.setAttribute('href', link.url);
      a.setAttribute('target', '_blank');
      a.setAttribute('rel', 'noopener');
      a.firstChild.nodeValue = linkLabel(link);
    });
  }

  function patchProject(doc, index, project, linksOnly) {
    var articles = doc.querySelectorAll('.projects-grid .project');
    var article = articles[index];
    if (!article) throw new Error('project selector missing');
    var links = article.querySelector('.project-links');
    if (!links) throw new Error('project links selector missing');
    if (linksOnly) {
      renderProjectLinks(links, project, true);
      return;
    }
    var icon = article.querySelector('.project-icon');
    var title = article.querySelector('h3');
    var description = article.querySelector('p');
    var tags = article.querySelector('.project-tags');
    var top = article.querySelector('.project-top');
    if (!icon || !title || !description || !tags || !top || !safeMarkup(project.desc)) throw new Error('unsafe or missing project projection');
    icon.textContent = project.icon || '•';
    title.textContent = project.title;
    description.innerHTML = project.desc;
    fillTags(tags, project.tags);
    var badge = top.querySelector('.badge');
    if (project.status) {
      if (!badge) {
        badge = article.ownerDocument.createElement('span');
        top.appendChild(badge);
      }
      badge.className = 'badge' + (project.isDev ? ' dev' : '');
      badge.textContent = project.status;
    } else if (badge) badge.remove();
    renderProjectLinks(links, project, false);
  }

  function buildHtml(baseHtml, model) {
    if (!baseHtml || !model) return baseHtml || '';
    var store = K.store;
    var isCurrent = baseHtml === store.baseHtml && model === store.model;
    if (isCurrent && buildCache.rev === store.rev && buildCache.baseHtml === baseHtml && buildCache.model === model) return buildCache.html;

    var doc = new DOMParser().parseFromString(baseHtml, 'text/html');
    applyName(doc, model);
    applyDescription(doc, model);
    applyTags(doc, model);
    applyContacts(doc, model, false);
    applyAvatar(doc, model);
    applyProjects(doc, model);
    applySeo(doc, model);

    var html = '<!DOCTYPE html>\n' + doc.documentElement.outerHTML;
    var phrases = model.phrases.length ? model.phrases : [model.name || 'Hello'];
    html = html.replace(/var phrases = \[[\s\S]*?\];/, 'var phrases = ' + JSON.stringify(phrases) + ';');
    var githubUser = userOf(model.github);
    var giteeUser = userOf(model.gitee);
    if (githubUser) html = html.replace(/https:\/\/api\.github\.com\/users\/[A-Za-z0-9_.-]+/g, 'https://api.github.com/users/' + githubUser);
    if (giteeUser) html = html.replace(/https:\/\/gitee\.com\/api\/v5\/users\/[A-Za-z0-9_.-]+/g, 'https://gitee.com/api/v5/users/' + giteeUser);
    html = html.replace(/projects: \d+/g, 'projects: ' + model.projects.length);

    if (isCurrent) buildCache = { rev: store.rev, baseHtml: baseHtml, model: model, html: html };
    K.debug.builds++;
    return html;
  }

  function sourceHtml() {
    return K.store.rawMode ? K.store.baseHtml : buildHtml(K.store.baseHtml, K.store.model);
  }

  function updateCounters() {
    var model = K.store.model;
    if (!model) return;
    $('#titleCount').textContent = model.seoTitle.length + ' / 60';
    $('#descCount').textContent = model.seoDesc.length + ' / 160';
  }

  function syncFields() {
    var model = K.store.model;
    if (!model) return;
    $('#fName').value = model.name;
    $('#fDesc').value = model.desc;
    $('#fPhrases').value = model.phrases.join('\n');
    $('#fTags').value = model.tags.join(', ');
    $('#fGithub').value = model.github;
    $('#fGitee').value = model.gitee;
    $('#fEmail').value = model.email;
    $('#seoTitle').value = model.seoTitle;
    $('#seoDesc').value = model.seoDesc;
    updateCounters();
  }

  function renderProjects() {
    var model = K.store.model;
    if (!model) return;
    $('#projectList').innerHTML = model.projects.map(function (project, index) {
      var collapsed = K.store.ui.collapsed.has(index);
      var links = project.links.map(function (link, linkIndex) {
        return '<div class="link-row" data-k="' + linkIndex + '">' +
          '<input class="l-text" value="' + attr(link.text) + '" placeholder="文字" aria-label="链接文字" />' +
          '<input class="l-url mono" value="' + attr(link.url) + '" placeholder="https://" aria-label="链接地址" />' +
          '<button type="button" class="icon-btn danger p-del-link" title="删除链接">✕</button>' +
          '</div>';
      }).join('');
      return '<div class="pcard' + (collapsed ? ' collapsed' : '') + '" data-i="' + index + '">' +
        '<div class="pcard-head">' +
          '<span class="p-grip" draggable="true" title="拖动调整顺序" aria-label="拖动排序">⠿</span>' +
          '<input class="p-icon" data-f="icon" value="' + attr(project.icon) + '" maxlength="3" aria-label="项目图标" />' +
          '<input class="p-title" data-f="title" value="' + attr(project.title) + '" placeholder="项目名称" aria-label="项目名称" />' +
          '<button type="button" class="icon-btn p-up" title="上移" aria-label="上移">↑</button>' +
          '<button type="button" class="icon-btn p-down" title="下移" aria-label="下移">↓</button>' +
          '<button type="button" class="icon-btn p-collapse" title="展开 / 收起">▾</button>' +
          '<button type="button" class="icon-btn danger p-del" title="删除项目">✕</button>' +
        '</div>' +
        '<div class="pcard-body">' +
          '<label class="field"><span>描述（支持简单 HTML）</span><textarea class="p-desc" data-f="desc" rows="3">' + esc(project.desc) + '</textarea></label>' +
          '<div class="row2">' +
            '<label class="field"><span>状态徽章</span><input class="p-status" data-f="status" value="' + attr(project.status) + '" placeholder="留空则不显示" /></label>' +
            '<label class="check"><input type="checkbox" class="p-dev"' + (project.isDev ? ' checked' : '') + ' /> 使用「开发中」样式</label>' +
          '</div>' +
          '<label class="field"><span>标签（逗号分隔）</span><input class="p-tags" data-f="tags" value="' + attr(project.tags.join(', ')) + '" /></label>' +
          '<div class="links-head"><span>链接</span><button type="button" class="text-btn p-add-link">+ 添加链接</button></div>' +
          '<div class="links">' + links + '</div>' +
        '</div>' +
      '</div>';
    }).join('');
  }

  function syncAvatar() {
    var model = K.store.model;
    if (!model) return;
    $('#avatarImg').src = '/preview/' + encodeURIComponent(model.avatar || 'avatar.png') + '?t=' + Date.now();
    $('#avatarName').textContent = model.avatar || 'avatar.png';
  }

  function updateRawCount() {
    var value = K.store.rawMode ? K.store.baseHtml : sourceHtml();
    $('#rawCount').textContent = (value ? value.split('\n').length : 0) + ' 行';
  }

  function refreshEditor() {
    syncFields();
    renderProjects();
    syncAvatar();
    if (K.store.ui.tab === 'raw') {
      $('#rawHtml').value = K.store.rawMode ? K.store.baseHtml : sourceHtml();
      updateRawCount();
    }
  }

  K.render = {
    parseSite: parseSite,
    buildHtml: buildHtml,
    sourceHtml: sourceHtml,
    projectHtml: projectHtml,
    projectLinksHtml: projectLinksHtml,
    safeMarkup: safeMarkup,
    applyName: applyName,
    applyDescription: applyDescription,
    applyTags: applyTags,
    applySeo: applySeo,
    applyContacts: applyContacts,
    patchProject: patchProject,
    syncFields: syncFields,
    renderProjects: renderProjects,
    syncAvatar: syncAvatar,
    updateCounters: updateCounters,
    updateRawCount: updateRawCount,
    refreshEditor: refreshEditor
  };
})(K);
