// rhwp Safari Web Extension - Content Script
// Safari Web Extension 표준 browser.* API 사용
// 웹페이지에서 HWP/HWPX 링크를 감지하고 뱃지 + 호버 카드 삽입

(() => {
  'use strict';

  const HWP_EXTENSIONS = /\.(hwp|hwpx)(\?.*)?$/i;
  const BADGE_CLASS = 'rhwp-badge';
  const HOVER_CLASS = 'rhwp-hover-card';
  const PROCESSED_ATTR = 'data-rhwp-processed';

  let settings = { autoOpen: true, showBadges: true, hoverPreview: true };

  // 설정 로드
  browser.runtime.sendMessage({ type: 'get-settings' }).then((result) => {
    if (result) settings = { ...settings, ...result };
    init();
  }).catch(() => {
    init();
  });

  function init() {
    if (settings.showBadges) {
      processLinks();
      observeDynamicContent();
    }
  }

  // 확장 존재 알림 (N-03: 허용 도메인에서만 노출, 버전 정보 제거)
  const ALLOWED_ANNOUNCE_DOMAINS = ['.go.kr', '.or.kr', '.ac.kr', '.mil.kr', '.korea.kr'];
  const shouldAnnounce = ALLOWED_ANNOUNCE_DOMAINS.some(d => location.hostname.endsWith(d));
  if (shouldAnnounce) {
    document.documentElement.setAttribute('data-hwp-extension', 'rhwp');
    window.dispatchEvent(new CustomEvent('hwp-extension-ready', {
      detail: { name: 'rhwp', capabilities: ['preview'] }
    }));
  }

  // ─── 유틸리티 ───

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  function extractFilename(anchor) {
    // URL에서 파일명 추출
    try {
      const pathname = new URL(anchor.href).pathname;
      const name = decodeURIComponent(pathname.split('/').pop() || '');
      if (HWP_EXTENSIONS.test(name)) return name;
    } catch { /* ignore */ }
    // 링크 텍스트 폴백
    const text = anchor.textContent.trim();
    return text || anchor.href;
  }

  function formatSize(bytes) {
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)}KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
  }

  // ─── 링크 감지 ───

  function isHwpLink(anchor) {
    if (!anchor.href) return false;
    if (anchor.getAttribute('data-hwp') === 'true') return true;
    return HWP_EXTENSIONS.test(anchor.href);
  }

  function createBadge(anchor) {
    const badge = document.createElement('span');
    badge.className = BADGE_CLASS;
    badge.title = browser.i18n.getMessage('badgeTooltip') || 'rhwp로 열기';

    badge.addEventListener('click', (e) => {
      e.preventDefault();
      e.stopPropagation();
      browser.runtime.sendMessage({
        type: 'open-hwp',
        url: anchor.href,
        filename: extractFilename(anchor)
      });
    });

    return badge;
  }

  // ─── 호버 미리보기 카드 ───

  let activeCard = null;
  let activeAnchor = null;
  let hoverTimeout = null;

  // 보안: 텍스트 길이 제한
  function truncate(str, max) {
    if (!str) return '';
    return str.length > max ? str.slice(0, max) + '…' : str;
  }

  // 보안: 안전한 이미지 URL인지 검증
  function isSafeImageUrl(url) {
    try {
      const parsed = new URL(url);
      return parsed.protocol === 'https:' || parsed.protocol === 'http:';
    } catch { return false; }
  }

  // DOM API로 안전하게 요소 생성 (innerHTML 미사용 — H-01 XSS 방어)
  function createDiv(className, text) {
    const div = document.createElement('div');
    div.className = className;
    if (text) div.textContent = text;
    return div;
  }

  function showHoverCard(anchor) {
    if (!settings.hoverPreview) return;
    if (activeAnchor === anchor && activeCard) return;

    hideHoverCard();

    const card = document.createElement('div');
    card.className = HOVER_CLASS;

    const title = anchor.getAttribute('data-hwp-title');
    const filename = extractFilename(anchor);

    if (title) {
      // 썸네일 (URL 스킴 검증)
      const thumbnail = anchor.getAttribute('data-hwp-thumbnail');
      if (thumbnail && isSafeImageUrl(thumbnail)) {
        const thumbContainer = document.createElement('div');
        thumbContainer.className = 'rhwp-hover-thumb';
        const img = document.createElement('img');
        img.src = thumbnail;
        img.alt = '미리보기';
        img.referrerPolicy = 'no-referrer';
        thumbContainer.appendChild(img);
        card.appendChild(thumbContainer);
      }

      card.appendChild(createDiv('rhwp-hover-title', truncate(title, 200)));

      // 메타 정보
      const meta = [];
      const format = anchor.getAttribute('data-hwp-format');
      const pages = anchor.getAttribute('data-hwp-pages');
      const size = anchor.getAttribute('data-hwp-size');
      if (format) meta.push(truncate(format.toUpperCase(), 10));
      if (pages) meta.push(`${truncate(pages, 10)}쪽`);
      if (size) meta.push(formatSize(Number(size)));
      if (meta.length > 0) {
        card.appendChild(createDiv('rhwp-hover-meta', meta.join(' · ')));
      }

      // 작성자/날짜
      const author = anchor.getAttribute('data-hwp-author');
      const date = anchor.getAttribute('data-hwp-date');
      if (author || date) {
        const info = [];
        if (author) info.push(truncate(author, 100));
        if (date) info.push(truncate(date, 20));
        card.appendChild(createDiv('rhwp-hover-info', info.join(' · ')));
      }

      // 카테고리
      const category = anchor.getAttribute('data-hwp-category');
      if (category) {
        card.appendChild(createDiv('rhwp-hover-category', truncate(category, 50)));
      }

      // 설명
      const description = anchor.getAttribute('data-hwp-description');
      if (description) {
        card.appendChild(createDiv('rhwp-hover-desc', truncate(description, 500)));
      }
    } else {
      // 기본 카드: 파일명 + 포맷
      const ext = filename.match(/\.(hwp|hwpx)$/i)?.[1]?.toUpperCase() || 'HWP';
      card.appendChild(createDiv('rhwp-hover-title', truncate(filename, 200)));
      card.appendChild(createDiv('rhwp-hover-meta', `${ext} 문서`));
    }

    card.appendChild(createDiv('rhwp-hover-action', '클릭하여 rhwp로 열기'));

    // 위치 계산: 링크 아래에 표시, 뷰포트 넘치면 위로
    const rect = anchor.getBoundingClientRect();
    const cardLeft = rect.left + window.scrollX;
    let cardTop = rect.bottom + window.scrollY + 4;

    // DOM에 추가하여 크기 측정
    card.style.visibility = 'hidden';
    card.style.left = `${cardLeft}px`;
    card.style.top = `${cardTop}px`;
    document.body.appendChild(card);

    const cardRect = card.getBoundingClientRect();
    if (cardRect.bottom > window.innerHeight) {
      cardTop = rect.top + window.scrollY - card.offsetHeight - 4;
    }
    const maxLeft = window.innerWidth + window.scrollX - card.offsetWidth - 8;
    card.style.left = `${Math.max(8, Math.min(cardLeft, maxLeft))}px`;
    card.style.top = `${Math.max(0, cardTop)}px`;
    card.style.visibility = '';

    activeCard = card;
    activeAnchor = anchor;

    card.addEventListener('mouseenter', () => clearTimeout(hoverTimeout));
    card.addEventListener('mouseleave', () => {
      hoverTimeout = setTimeout(() => hideHoverCard(), 150);
    });
    card.addEventListener('click', () => {
      hideHoverCard();
      browser.runtime.sendMessage({
        type: 'open-hwp',
        url: anchor.href,
        filename: extractFilename(anchor)
      });
    });
  }

  function hideHoverCard() {
    clearTimeout(hoverTimeout);
    if (activeCard) {
      activeCard.remove();
      activeCard = null;
      activeAnchor = null;
    }
  }

  function attachHoverEvents(anchor) {
    if (!settings.hoverPreview) return;
    anchor.addEventListener('mouseenter', () => {
      clearTimeout(hoverTimeout);
      hoverTimeout = setTimeout(() => showHoverCard(anchor), 250);
    });
    anchor.addEventListener('mouseleave', () => {
      clearTimeout(hoverTimeout);
      hoverTimeout = setTimeout(() => hideHoverCard(), 150);
    });
  }

  // ─── HWP 링크 클릭 가로채기 ───
  // Safari는 downloads API가 없으므로, HWP 링크 클릭 시 뷰어도 함께 연다.
  // 다운로드는 정상 진행 (preventDefault 하지 않음)

  function interceptHwpClick(anchor) {
    if (!settings.autoOpen) return;
    anchor.addEventListener('click', () => {
      browser.runtime.sendMessage({
        type: 'open-hwp',
        url: anchor.href,
        filename: extractFilename(anchor)
      });
    });
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

      interceptHwpClick(anchor);
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
