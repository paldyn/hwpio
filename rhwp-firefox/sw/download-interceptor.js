// 다운로드 가로채기 (Firefox 버전)
// - onCreated: URL 기반 즉시 감지 (1차 판정)
// - onChanged: filename 확정 시 재판정 (2차 판정)
// - browser.downloads.search로 최신 DownloadItem 재조회
// - handled 집합으로 동일 다운로드 중복 처리 방지

import { openViewer } from './viewer-launcher.js';

const HWP_EXTENSIONS = /\.(hwp|hwpx)(\?.*)?$/i;
const candidates = new Set();  // 1차 미판정 downloadId
const handled = new Set();     // 이미 처리된 downloadId

export function setupDownloadInterceptor() {
  // 1차: 다운로드 시작 시 URL 기반 즉시 감지
  browser.downloads.onCreated.addListener((item) => {
    if (handled.has(item.id)) return;

    if (HWP_EXTENSIONS.test(item.url || '')) {
      handled.add(item.id);
      handleHwpDownload(item);
    } else {
      // URL로 판별 불가 → 후보 등록, filename 확정 대기
      candidates.add(item.id);
    }
  });

  // 2차: filename 확정 시 재판정
  browser.downloads.onChanged.addListener(async (delta) => {
    if (handled.has(delta.id)) return;

    if (delta.filename?.current && HWP_EXTENSIONS.test(delta.filename.current)) {
      handled.add(delta.id);
      candidates.delete(delta.id);

      // 최신 DownloadItem 재조회 (url, fileSize 등 완전한 정보 확보)
      const [item] = await browser.downloads.search({ id: delta.id });
      if (item) {
        handleHwpDownload(item);
      }
    }

    // 완료/에러 시 후보 정리
    if (delta.state?.current === 'complete' || delta.error) {
      candidates.delete(delta.id);
      setTimeout(() => handled.delete(delta.id), 30000);
    }
  });
}

async function handleHwpDownload(item) {
  try {
    const settings = await browser.storage.sync.get({ autoOpen: true });
    if (!settings.autoOpen) return;

    if (item.fileSize > 50 * 1024 * 1024) {
      console.warn(
        `[rhwp] 대용량 파일: ${item.filename} (${(item.fileSize / 1024 / 1024).toFixed(1)}MB)`
      );
    }

    openViewer({
      url: item.url,
      filename: item.filename
    });
  } catch (err) {
    console.error('[rhwp] 다운로드 인터셉터 오류:', err);
  }
}
