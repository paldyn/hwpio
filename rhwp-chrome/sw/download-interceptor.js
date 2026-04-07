// 다운로드 가로채기
// - .hwp/.hwpx 다운로드 감지 → 뷰어로 열기
// - 사용자 설정(autoOpen)에 따라 동작

import { openViewer } from './viewer-launcher.js';

const HWP_EXTENSIONS = /\.(hwp|hwpx)$/i;

/**
 * 다운로드 인터셉터를 설정한다.
 */
export function setupDownloadInterceptor() {
  chrome.downloads.onDeterminingFilename.addListener((item, suggest) => {
    const filename = item.filename || '';

    if (HWP_EXTENSIONS.test(filename)) {
      handleHwpDownload(item);
    }

    // 기본 파일명 유지 (다운로드는 정상 진행)
    suggest({ filename: item.filename });
  });
}

async function handleHwpDownload(item) {
  try {
    const settings = await chrome.storage.sync.get({ autoOpen: true });

    if (settings.autoOpen) {
      // 대용량 파일 경고 (50MB 초과)
      if (item.fileSize > 50 * 1024 * 1024) {
        console.warn(`[rhwp] 대용량 파일: ${item.filename} (${(item.fileSize / 1024 / 1024).toFixed(1)}MB)`);
      }

      openViewer({
        url: item.url,
        filename: item.filename
      });
    }
  } catch (err) {
    console.error('[rhwp] 다운로드 인터셉터 오류:', err);
  }
}
