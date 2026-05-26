import type { PageInfo } from '@/core/types';
import type { GridViewSettings } from './grid-settings';

const MM_TO_PX = 96 / 25.4;

export function createGridOverlay(
  pageIdx: number,
  pageInfo: PageInfo,
  zoom: number,
  settings: GridViewSettings,
): HTMLDivElement {
  const overlay = document.createElement('div');
  overlay.className = 'page-grid-overlay';
  overlay.dataset.rhwpGridPage = String(pageIdx);
  overlay.style.backgroundImage = buildBackgroundImage(settings);
  overlay.style.backgroundSize = `${settings.horizontalMm * MM_TO_PX * zoom}px ${settings.verticalMm * MM_TO_PX * zoom}px`;
  overlay.style.backgroundPosition = buildBackgroundPosition(pageInfo, zoom, settings);
  overlay.style.clipPath = buildClipPath(pageInfo, zoom, settings);
  overlay.style.zIndex = settings.layer === 'inFrontOfText' ? '4' : '1';
  overlay.style.opacity = settings.layer === 'inFrontOfText' ? '0.72' : '0.5';
  return overlay;
}

export function applyGridOverlayBox(
  overlay: HTMLElement,
  canvas: HTMLCanvasElement,
): void {
  overlay.style.position = 'absolute';
  overlay.style.top = canvas.style.top;
  overlay.style.left = canvas.style.left;
  overlay.style.transform = canvas.style.transform;
  overlay.style.width = canvas.style.width;
  overlay.style.height = canvas.style.height;
  overlay.style.pointerEvents = 'none';
  overlay.style.overflow = 'hidden';
}

function buildBackgroundImage(settings: GridViewSettings): string {
  const color = 'rgba(35, 103, 255, 0.72)';
  switch (settings.pattern) {
    case 'horizontal':
      return `linear-gradient(to bottom, ${color} 0, ${color} 1px, transparent 1px)`;
    case 'vertical':
      return `linear-gradient(to right, ${color} 0, ${color} 1px, transparent 1px)`;
    case 'both':
      return [
        `linear-gradient(to right, ${color} 0, ${color} 1px, transparent 1px)`,
        `linear-gradient(to bottom, ${color} 0, ${color} 1px, transparent 1px)`,
      ].join(', ');
    case 'dots':
    default:
      return `radial-gradient(circle, ${color} 0 1px, transparent 1.2px)`;
  }
}

function buildBackgroundPosition(
  pageInfo: PageInfo,
  zoom: number,
  settings: GridViewSettings,
): string {
  const origin = getGridOriginPx(pageInfo, settings);
  const patternOffset = getPatternOffsetPx(settings);
  const x = (origin.x + settings.offsetXmm * MM_TO_PX + patternOffset.x) * zoom;
  const y = (origin.y + settings.offsetYmm * MM_TO_PX + patternOffset.y) * zoom;
  return `${x}px ${y}px`;
}

function buildClipPath(
  pageInfo: PageInfo,
  zoom: number,
  settings: GridViewSettings,
): string {
  if (settings.origin === 'paper') return 'none';

  const pageArea = getPageGridAreaPx(pageInfo);
  const left = pageArea.left * zoom;
  const top = pageArea.top * zoom;
  const right = pageArea.right * zoom;
  const bottom = pageArea.bottom * zoom;
  return `inset(${top}px ${right}px ${bottom}px ${left}px)`;
}

function getGridOriginPx(
  pageInfo: PageInfo,
  settings: GridViewSettings,
): { x: number; y: number } {
  if (settings.origin === 'paper') {
    return { x: 0, y: 0 };
  }
  const pageArea = getPageGridAreaPx(pageInfo);
  return {
    x: pageArea.left,
    y: pageArea.top,
  };
}

function getPageGridAreaPx(pageInfo: PageInfo): { left: number; right: number; top: number; bottom: number } {
  return {
    left: pageInfo.pageBorderLeft ?? pageInfo.marginLeft,
    right: pageInfo.pageBorderRight ?? pageInfo.marginRight,
    top: pageInfo.pageBorderTop ?? pageInfo.marginTop,
    bottom: pageInfo.pageBorderBottom ?? pageInfo.marginBottom,
  };
}

function getPatternOffsetPx(settings: GridViewSettings): { x: number; y: number } {
  if (settings.pattern !== 'dots') {
    return { x: 0, y: 0 };
  }
  return {
    x: settings.horizontalMm * MM_TO_PX / 2,
    y: settings.verticalMm * MM_TO_PX / 2,
  };
}
