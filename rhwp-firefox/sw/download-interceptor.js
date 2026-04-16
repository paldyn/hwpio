// 다운로드 가로채기 (Firefox 버전)
// - onCreated: URL 기반 즉시 감지 (1차 판정)
// - onChanged: filename 확정 시 재판정 (2차 판정)
// - browser.downloads.search로 최신 DownloadItem 재조회
// - handled 집합으로 동일 다운로드 중복 처리 방지

import { openViewer } from './viewer-launcher.js';

const HWP_EXTENSIONS = /\.(hwp|hwpx)(\?.*)?$/i;
const handled = new Set();     // 이미 처리된 downloadId

export function setupDownloadInterceptor() {
  // 1차: 다운로드 시작 시 URL 기반 즉시 감지
  browser.downloads.onCreated.addListener((item) => {
    if (handled.has(item.id)) return;

    if (HWP_EXTENSIONS.test(item.url || '')) {
      handled.add(item.id);
      handleHwpDownload(item);
    }
    // URL로 판별 불가 시: filename 확정될 때 onChanged에서 재판정
  });

  // 2차: filename 확정 시 재판정
  browser.downloads.onChanged.addListener(async (delta) => {
    // filename 확정으로 HWP가 처음 판정되는 경우에만 처리
    if (!handled.has(delta.id)
      && delta.filename?.current
      && HWP_EXTENSIONS.test(delta.filename.current)) {
      handled.add(delta.id);

      try {
        // 최신 DownloadItem 재조회 (url, fileSize 등 완전한 정보 확보)
        const [item] = await browser.downloads.search({ id: delta.id });
        if (item) {
          handleHwpDownload(item);
        }
      } catch (err) {
        console.error('[rhwp] 다운로드 항목 재조회 오류:', err);
      }
    }

    // 완료/에러 시 handled 정리 (메모리 누수 방지)
    // onCreated/onChanged 양쪽 경로에서 들어간 id 모두에 대해 cleanup 보장
    if (handled.has(delta.id) && (delta.state?.current === 'complete' || delta.error)) {
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
