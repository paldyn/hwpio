/**
 * [Task #1 · HanPage 데스크톱] 네이티브(Tauri)↔웹 파일 핸드오프 브리지.
 *
 * **web-inert 원칙**: 이 모듈은 브라우저(GitHub Pages 웹 빌드)에서 완전한 no-op 이다.
 * - `initDesktopBridge()` 는 데스크톱 런타임이 아니면 즉시 return → 아무 핸들러도 등록 안 함.
 * - `getDesktopOpenHandler()` / `getDesktopSaveHandler()` 는 브라우저에서 항상 `null` →
 *   `file.ts` 의 가드 분기가 건너뛰어 기존 웹 동작이 그대로 유지된다.
 * - `@tauri-apps/*` npm 의존성을 일절 import 하지 않는다. Tauri 웹뷰에만 주입되는 전역
 *   `window.__TAURI__`(tauri.conf.json `withGlobalTauri: true`)를 통해서만 네이티브를 호출한다.
 *   따라서 웹 번들에 Tauri 코드가 정적으로도 동적으로도 포함되지 않는다.
 */

/** Tauri `invoke` 시그니처(필요한 부분만 로컬 선언 — @tauri-apps/api 의존 회피). */
type TauriInvoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

interface TauriGlobal {
  core?: { invoke?: TauriInvoke };
}

/** Tauri 웹뷰 여부. 데스크톱 런타임에서만 `__TAURI_INTERNALS__` 가 주입된다. */
export function isDesktopRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function tauriInvoke(): TauriInvoke | null {
  const g = (window as unknown as { __TAURI__?: TauriGlobal }).__TAURI__;
  return g?.core?.invoke ?? null;
}

// ─── 열기 ──────────────────────────────────────────────────────────────────
/** 네이티브 열기로 받은 바이트를 기존 문서 오픈 흐름(eventBus)에 흘려보내는 콜백. */
export type EmitOpenDocument = (bytes: Uint8Array, fileName: string) => void;
/** `file:open` 에서 호출. 네이티브 dialog → 바이트 → `emit` 으로 문서 오픈. */
export type DesktopOpenHandler = (emit: EmitOpenDocument) => Promise<void>;

interface OpenedFile {
  name: string;
  path: string;
  data: number[];
}

// ─── 저장 ──────────────────────────────────────────────────────────────────
export interface DesktopSaveRequest {
  bytes: Uint8Array;
  suggestedName: string;
  saveAs: boolean;
}
export type DesktopSaveResult =
  | { status: 'saved'; fileName: string }
  | { status: 'cancelled' }
  | { status: 'failed'; message: string };
export type DesktopSaveHandler = (req: DesktopSaveRequest) => Promise<DesktopSaveResult>;

type SaveOutcome =
  | { status: 'saved'; path: string; name: string }
  | { status: 'cancelled' };

// 데스크톱에서만 채워지는 핸들러. 브라우저에서는 영구히 null(= web-inert).
let openHandler: DesktopOpenHandler | null = null;
let saveHandler: DesktopSaveHandler | null = null;

/**
 * 브리지 초기화. `main.ts` 의 `initialize()` 에서 1줄로 호출한다.
 * 브라우저에서는 즉시 return 하여 완전한 no-op 이다.
 */
export function initDesktopBridge(): void {
  if (!isDesktopRuntime()) return; // 웹: no-op

  const invoke = tauriInvoke();
  if (!invoke) {
    console.warn('[desktop-bridge] __TAURI__ invoke 미가용 — 네이티브 연동 비활성');
    return;
  }

  openHandler = async (emit) => {
    const res = await invoke<OpenedFile | null>('cmd_open_document');
    if (!res) return; // 사용자 취소
    emit(new Uint8Array(res.data), res.name);
  };

  saveHandler = async (req) => {
    try {
      const res = await invoke<SaveOutcome>('cmd_save_document', {
        suggestedName: req.suggestedName,
        data: Array.from(req.bytes),
      });
      if (res.status === 'cancelled') return { status: 'cancelled' };
      return { status: 'saved', fileName: res.name };
    } catch (e) {
      return { status: 'failed', message: e instanceof Error ? e.message : String(e) };
    }
  };
}

export function getDesktopOpenHandler(): DesktopOpenHandler | null {
  return openHandler;
}

export function getDesktopSaveHandler(): DesktopSaveHandler | null {
  return saveHandler;
}
