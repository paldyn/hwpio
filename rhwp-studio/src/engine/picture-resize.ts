/**
 * Shift+방향키 개체 크기 조절 계산 (#1231) — 순수 함수 모듈.
 *
 * input-handler-picture.ts 의 resizeSelectedPicture 가 사용하며,
 * node:test 단위 테스트(tests/picture-resize.test.ts) 대상이라
 * 외부 import 없이 유지한다 (navigation-keymap.ts 와 동일 패턴).
 */

/** 개체 최소 크기 (≈1mm). input-handler-picture.ts 의 드래그 리사이즈 하한과 동일 값. */
export const MIN_SIZE_HWP = 283;

export type ArrowKey = 'ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight';

/** 방향키 → 가로/세로 크기 증감 (한컴 정합: →/← 가로 증감, ↓/↑ 세로 증감) */
export function arrowResizeDelta(key: ArrowKey, step: number): { deltaW: number; deltaH: number } {
  switch (key) {
    case 'ArrowRight': return { deltaW: step, deltaH: 0 };
    case 'ArrowLeft':  return { deltaW: -step, deltaH: 0 };
    case 'ArrowDown':  return { deltaW: 0, deltaH: step };
    case 'ArrowUp':    return { deltaW: 0, deltaH: -step };
  }
}

export interface ArrowResizeResult {
  before: { width: number; height: number };
  after: { width: number; height: number };
}

/**
 * 현재 크기에 방향키 증감을 적용해 최소 크기로 클램프한 before/after 를 돌려준다.
 * 변화가 없으면(이미 최소 크기에서 더 줄이려는 경우) null.
 */
export function computeArrowResize(
  key: ArrowKey,
  width: number,
  height: number,
  step: number,
): ArrowResizeResult | null {
  if (!Number.isFinite(width) || !Number.isFinite(height) || step <= 0) return null;
  const { deltaW, deltaH } = arrowResizeDelta(key, step);
  const newW = Math.max(Math.round(width + deltaW), MIN_SIZE_HWP);
  const newH = Math.max(Math.round(height + deltaH), MIN_SIZE_HWP);
  if (newW === width && newH === height) return null;
  return {
    before: { width, height },
    after: { width: newW, height: newH },
  };
}
