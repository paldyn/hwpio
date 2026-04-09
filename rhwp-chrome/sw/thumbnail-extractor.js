// HWP 파일에서 PrvImage 썸네일을 경량 추출 (WASM 불필요)
//
// CFB(OLE2 Compound File) 컨테이너에서 /PrvImage 스트림만 추출한다.
// 전체 HWP 파싱 없이 썸네일만 빠르게 얻을 수 있다.

const THUMBNAIL_CACHE = new Map();
const CACHE_MAX_SIZE = 100;

/**
 * URL에서 HWP 파일을 fetch하여 PrvImage 썸네일을 추출한다.
 * @param {string} url - HWP 파일 URL
 * @returns {Promise<{dataUri: string, width: number, height: number} | null>}
 */
export async function extractThumbnailFromUrl(url) {
  // 캐시 확인
  if (THUMBNAIL_CACHE.has(url)) {
    return THUMBNAIL_CACHE.get(url);
  }

  try {
    const response = await fetch(url);
    if (!response.ok) return null;
    const buffer = await response.arrayBuffer();
    const data = new Uint8Array(buffer);

    const result = extractPrvImage(data);
    if (result) {
      // 캐시 저장 (LRU)
      if (THUMBNAIL_CACHE.size >= CACHE_MAX_SIZE) {
        const firstKey = THUMBNAIL_CACHE.keys().next().value;
        THUMBNAIL_CACHE.delete(firstKey);
      }
      THUMBNAIL_CACHE.set(url, result);
    }
    return result;
  } catch {
    return null;
  }
}

/**
 * CFB 바이너리에서 /PrvImage 스트림을 추출한다.
 *
 * CFB 구조:
 * - 헤더 512바이트 (매직: D0 CF 11 E0 A1 B1 1A E1)
 * - 디렉토리 엔트리에서 "PrvImage" 이름을 찾아 스트림 위치/크기 파악
 * - 해당 섹터 체인을 따라 데이터 읽기
 *
 * 간소화 구현: 바이너리에서 이미지 시그니처를 직접 탐색
 */
function extractPrvImage(data) {
  // CFB 매직 넘버 확인
  if (data.length < 512) return null;
  if (data[0] !== 0xD0 || data[1] !== 0xCF || data[2] !== 0x11 || data[3] !== 0xE0) return null;

  // CFB 헤더에서 섹터 크기 읽기
  const sectorSizePow = data[30] | (data[31] << 8);
  const sectorSize = 1 << sectorSizePow; // 보통 512

  // 디렉토리 엔트리에서 "PrvImage" 찾기
  // CFB 첫 번째 디렉토리 섹터 위치: 헤더 offset 48 (4바이트 LE)
  const dirStartSector = readU32LE(data, 48);
  const dirOffset = (dirStartSector + 1) * sectorSize;

  // 디렉토리 엔트리 순회 (각 128바이트)
  for (let i = 0; i < 128; i++) { // 최대 128개 엔트리 탐색
    const entryOffset = dirOffset + i * 128;
    if (entryOffset + 128 > data.length) break;

    // 엔트리 이름 읽기 (UTF-16LE)
    const nameLen = readU16LE(data, entryOffset + 64); // 바이트 단위 이름 길이
    if (nameLen === 0 || nameLen > 64) continue;

    const name = readUTF16LE(data, entryOffset, nameLen);
    if (name !== 'PrvImage') continue;

    // 스트림 시작 섹터와 크기
    const startSector = readU32LE(data, entryOffset + 116);
    const streamSize = readU32LE(data, entryOffset + 120);

    if (streamSize === 0 || streamSize > 10 * 1024 * 1024) continue; // 10MB 제한

    // FAT 체인을 따라 데이터 읽기
    const streamData = readStreamFromFAT(data, startSector, streamSize, sectorSize);
    if (!streamData) continue;

    return parseImageData(streamData);
  }

  return null;
}

/**
 * FAT 체인을 따라 스트림 데이터를 읽는다.
 */
function readStreamFromFAT(data, startSector, streamSize, sectorSize) {
  // FAT (File Allocation Table) 읽기
  // FAT 시작: 헤더 offset 44에 DIFAT 첫 번째 엔트리 → 실제로는 헤더 offset 76부터 109개 DIFAT 엔트리
  const fatSectors = [];
  for (let i = 0; i < 109; i++) {
    const fatSect = readU32LE(data, 76 + i * 4);
    if (fatSect === 0xFFFFFFFE || fatSect === 0xFFFFFFFF) break;
    fatSectors.push(fatSect);
  }

  // FAT 테이블 구성
  const fatEntries = [];
  for (const fs of fatSectors) {
    const fatOffset = (fs + 1) * sectorSize;
    const entriesPerSector = sectorSize / 4;
    for (let j = 0; j < entriesPerSector; j++) {
      const off = fatOffset + j * 4;
      if (off + 4 > data.length) break;
      fatEntries.push(readU32LE(data, off));
    }
  }

  // 섹터 체인을 따라 데이터 수집
  const result = new Uint8Array(streamSize);
  let sector = startSector;
  let bytesRead = 0;

  for (let safety = 0; safety < 10000 && bytesRead < streamSize; safety++) {
    if (sector >= 0xFFFFFFFE) break;
    const offset = (sector + 1) * sectorSize;
    const copyLen = Math.min(sectorSize, streamSize - bytesRead);
    if (offset + copyLen > data.length) break;
    result.set(data.subarray(offset, offset + copyLen), bytesRead);
    bytesRead += copyLen;

    // 다음 섹터
    if (sector < fatEntries.length) {
      sector = fatEntries[sector];
    } else {
      break;
    }
  }

  return bytesRead >= streamSize ? result : null;
}

/**
 * 이미지 데이터에서 포맷 감지 + dataUri 생성
 */
function parseImageData(data) {
  let mime, width = 0, height = 0;

  if (data.length >= 8 && data[0] === 0x89 && data[1] === 0x50 && data[2] === 0x4E && data[3] === 0x47) {
    // PNG
    mime = 'image/png';
    if (data.length >= 24) {
      width = (data[16] << 24) | (data[17] << 16) | (data[18] << 8) | data[19];
      height = (data[20] << 24) | (data[21] << 16) | (data[22] << 8) | data[23];
    }
  } else if (data.length >= 2 && data[0] === 0x42 && data[1] === 0x4D) {
    // BMP
    mime = 'image/bmp';
    if (data.length >= 26) {
      width = readU32LE(data, 18);
      height = Math.abs(readI32LE(data, 22));
    }
  } else if (data.length >= 3 && data[0] === 0x47 && data[1] === 0x49 && data[2] === 0x46) {
    // GIF
    mime = 'image/gif';
    if (data.length >= 10) {
      width = readU16LE(data, 6);
      height = readU16LE(data, 8);
    }
  } else {
    return null;
  }

  // Base64 인코딩
  let binary = '';
  for (let i = 0; i < data.length; i++) {
    binary += String.fromCharCode(data[i]);
  }
  const base64 = btoa(binary);
  const dataUri = `data:${mime};base64,${base64}`;

  return { dataUri, width, height, mime };
}

// ─── 바이너리 헬퍼 ───

function readU16LE(data, offset) {
  return data[offset] | (data[offset + 1] << 8);
}

function readU32LE(data, offset) {
  return (data[offset] | (data[offset + 1] << 8) | (data[offset + 2] << 16) | (data[offset + 3] << 24)) >>> 0;
}

function readI32LE(data, offset) {
  return data[offset] | (data[offset + 1] << 8) | (data[offset + 2] << 16) | (data[offset + 3] << 24);
}

function readUTF16LE(data, offset, byteLen) {
  let str = '';
  for (let i = 0; i < byteLen - 2; i += 2) {
    const code = data[offset + i] | (data[offset + i + 1] << 8);
    if (code === 0) break;
    str += String.fromCharCode(code);
  }
  return str;
}
