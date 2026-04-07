// rhwp Chrome Extension - Content Script
// 웹페이지에서 HWP/HWPX 링크를 자동 감지하고 rhwp 아이콘을 삽입

(() => {
  'use strict';

  const HWP_EXTENSIONS = /\.(hwp|hwpx)(\?.*)?$/i;
  const BADGE_CLASS = 'rhwp-badge';
  const HOVER_CLASS = 'rhwp-hover-card';
  const PROCESSED_ATTR = 'data-rhwp-processed';

  // 사용자 설정 (Service Worker에서 로드)
  let settings = { showBadges: true, hoverPreview: true };

  chrome.runtime.sendMessage({ type: 'get-settings' }, (result) => {
    if (result) settings = result;
    // 설정 로드 후 초기 스캔
    if (settings.showBadges) {
      processLinks();
      observeDynamicContent();
    }
  });

  // 확장 존재 알림
  document.documentElement.setAttribute('data-hwp-extension', 'rhwp');
  document.documentElement.setAttribute('data-hwp-extension-version', '0.1.0');
  window.dispatchEvent(new CustomEvent('hwp-extension-ready', {
    detail: { name: 'rhwp', version: '0.1.0', capabilities: ['preview', 'edit', 'print'] }
  }));

  // 개발자 도구 주입 (페이지 컨텍스트에 rhwpDev 노출)
  const devScript = document.createElement('script');
  devScript.src = chrome.runtime.getURL('dev-tools-inject.js');
  (document.head || document.documentElement).appendChild(devScript);
  devScript.onload = () => devScript.remove();

  // ─── 링크 감지 ───

  function isHwpLink(anchor) {
    if (!anchor.href) return false;
    if (anchor.getAttribute('data-hwp') === 'true') return true;
    return HWP_EXTENSIONS.test(anchor.href);
  }

  function createBadge(anchor) {
    const badge = document.createElement('span');
    badge.className = BADGE_CLASS;

    const title = anchor.getAttribute('data-hwp-title');
    const pages = anchor.getAttribute('data-hwp-pages');
    const size = anchor.getAttribute('data-hwp-size');

    let tooltip;
    if (title && pages && size) {
      tooltip = chrome.i18n.getMessage('badgeTooltipWithInfo', [title, pages, formatSize(Number(size))]);
    } else if (title) {
      tooltip = title;
    } else {
      tooltip = chrome.i18n.getMessage('badgeTooltip');
    }
    badge.title = tooltip;

    badge.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      chrome.runtime.sendMessage({ type: 'open-hwp', url: anchor.href });
    });

    return badge;
  }

  // ─── 호버 미리보기 카드 ───

  let activeCard = null;
  let hoverTimeout = null;

  function showHoverCard(anchor) {
    if (!settings.hoverPreview) return;
    // data-hwp-* 메타데이터가 있는 경우에만 카드 표시
    const title = anchor.getAttribute('data-hwp-title');
    if (!title) return;

    hideHoverCard();

    const card = document.createElement('div');
    card.className = HOVER_CLASS;

    const pages = anchor.getAttribute('data-hwp-pages');
    const size = anchor.getAttribute('data-hwp-size');
    const author = anchor.getAttribute('data-hwp-author');
    const date = anchor.getAttribute('data-hwp-date');
    const category = anchor.getAttribute('data-hwp-category');
    const description = anchor.getAttribute('data-hwp-description');
    const format = anchor.getAttribute('data-hwp-format');
    const thumbnail = anchor.getAttribute('data-hwp-thumbnail');

    let html = '';

    if (thumbnail) {
      html += `<div class="rhwp-hover-thumb"><img src="${thumbnail}" alt="미리보기"></div>`;
    }

    html += `<div class="rhwp-hover-title">${title}</div>`;

    const meta = [];
    if (format) meta.push(format.toUpperCase());
    if (pages) meta.push(`${pages}쪽`);
    if (size) meta.push(formatSize(Number(size)));
    if (meta.length > 0) {
      html += `<div class="rhwp-hover-meta">${meta.join(' · ')}</div>`;
    }

    if (author || date) {
      const info = [];
      if (author) info.push(author);
      if (date) info.push(date);
      html += `<div class="rhwp-hover-info">${info.join(' · ')}</div>`;
    }

    if (category) {
      html += `<div class="rhwp-hover-category">${category}</div>`;
    }

    if (description) {
      html += `<div class="rhwp-hover-desc">${description}</div>`;
    }

    html += `<div class="rhwp-hover-action">클릭하여 rhwp로 열기</div>`;

    card.innerHTML = html;

    // 위치 계산
    const rect = anchor.getBoundingClientRect();
    card.style.left = `${rect.left + window.scrollX}px`;
    card.style.top = `${rect.bottom + window.scrollY + 4}px`;

    document.body.appendChild(card);
    activeCard = card;

    // 카드에 마우스 올리면 유지
    card.addEventListener('mouseenter', () => clearTimeout(hoverTimeout));
    card.addEventListener('mouseleave', () => hideHoverCard());
  }

  function hideHoverCard() {
    if (activeCard) {
      activeCard.remove();
      activeCard = null;
    }
    clearTimeout(hoverTimeout);
  }

  function attachHoverEvents(anchor) {
    if (!settings.hoverPreview) return;

    anchor.addEventListener('mouseenter', () => {
      hoverTimeout = setTimeout(() => showHoverCard(anchor), 300);
    });
    anchor.addEventListener('mouseleave', () => {
      hoverTimeout = setTimeout(() => hideHoverCard(), 200);
    });
  }

  // ─── 유틸리티 ───

  function formatSize(bytes) {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)}KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  }

  // ─── 링크 처리 ───

  function processLinks(root = document) {
    const anchors = root.querySelectorAll('a[href]');
    for (const anchor of anchors) {
      if (anchor.hasAttribute(PROCESSED_ATTR)) continue;
      if (!isHwpLink(anchor)) continue;

      anchor.setAttribute(PROCESSED_ATTR, 'true');

      if (settings.showBadges) {
        const badge = createBadge(anchor);
        anchor.style.position = anchor.style.position || 'relative';
        anchor.insertAdjacentElement('afterend', badge);
      }

      attachHoverEvents(anchor);
    }
  }

  function observeDynamicContent() {
    const observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        for (const node of mutation.addedNodes) {
          if (node.nodeType === Node.ELEMENT_NODE) {
            processLinks(node);
          }
        }
      }
    });
    observer.observe(document.body, { childList: true, subtree: true });
  }
})();
