/** 키보드 단축키 정의 */
export interface ShortcutDef {
  /** 키 문자 (소문자). 예: 'z', 'b', '=', '-' */
  key: string;
  /** Ctrl (Windows) 또는 Meta (Mac) */
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
}

/** 기본 단축키 → 커맨드 ID 매핑 */
export const defaultShortcuts: [ShortcutDef, string][] = [
  // 편집
  [{ key: 'z', ctrl: true }, 'edit:undo'],
  [{ key: 'z', ctrl: true, shift: true }, 'edit:redo'],
  [{ key: 'y', ctrl: true }, 'edit:redo'],
  [{ key: 'a', ctrl: true }, 'edit:select-all'],

  // 파일
  [{ key: 'n', alt: true }, 'file:new-doc'],
  [{ key: 'ㅜ', alt: true }, 'file:new-doc'],
  [{ key: 's', ctrl: true }, 'file:save'],
  [{ key: 'p', ctrl: true }, 'file:print'],

  // 서식
  [{ key: 'b', ctrl: true }, 'format:bold'],
  [{ key: 'i', ctrl: true }, 'format:italic'],
  [{ key: 'u', ctrl: true }, 'format:underline'],
  [{ key: 'l', alt: true }, 'format:char-shape'],
  [{ key: 'ㄹ', alt: true }, 'format:char-shape'],
  [{ key: 't', alt: true }, 'format:para-shape'],
  [{ key: 'ㅅ', alt: true }, 'format:para-shape'],

  // 서식 – 스타일
  [{ key: 'f6' }, 'format:style-dialog'],

  // 쪽
  [{ key: 'f7' }, 'file:page-setup'],

  // 줌
  [{ key: '=', ctrl: true }, 'view:zoom-in'],
  [{ key: '+', ctrl: true }, 'view:zoom-in'],
  [{ key: '-', ctrl: true }, 'view:zoom-out'],
  [{ key: '0', ctrl: true }, 'view:zoom-100'],

  // 검색
  [{ key: 'f', ctrl: true }, 'edit:find'],
  [{ key: 'f2', ctrl: true }, 'edit:find-replace'],
  [{ key: 'l', ctrl: true }, 'edit:find-again'],
  [{ key: 'g', alt: true }, 'edit:goto'],
  [{ key: 'ㅎ', alt: true }, 'edit:goto'],

  // 입력
  [{ key: 'f10', alt: true }, 'insert:symbols'],

  // 쪽
  [{ key: 'enter', ctrl: true }, 'page:break'],
  [{ key: 'enter', ctrl: true, shift: true }, 'page:column-break'],

  // 줄간격
  [{ key: 'a', alt: true, shift: true }, 'format:line-spacing-decrease'],
  [{ key: 'ㅁ', alt: true, shift: true }, 'format:line-spacing-decrease'],
  [{ key: 'z', alt: true, shift: true }, 'format:line-spacing-increase'],
  [{ key: 'ㅋ', alt: true, shift: true }, 'format:line-spacing-increase'],

  // 글꼴 크기
  [{ key: 'e', alt: true, shift: true }, 'format:font-size-increase'],
  [{ key: 'ㄷ', alt: true, shift: true }, 'format:font-size-increase'],
  [{ key: 'r', alt: true, shift: true }, 'format:font-size-decrease'],
  [{ key: 'ㄱ', alt: true, shift: true }, 'format:font-size-decrease'],

  // 표
  [{ key: 'insert', alt: true }, 'table:insert-col-left'],
  [{ key: 'delete', alt: true }, 'table:delete-col'],
];

/**
 * KeyboardEvent에 매칭되는 단축키가 있으면 커맨드 ID를 반환한다.
 * 없으면 null.
 */
export function matchShortcut(
  e: KeyboardEvent,
  shortcuts: [ShortcutDef, string][],
): string | null {
  const ctrlOrMeta = e.ctrlKey || e.metaKey;

  for (const [def, cmdId] of shortcuts) {
    if (def.ctrl && !ctrlOrMeta) continue;
    if (!def.ctrl && ctrlOrMeta) continue;
    if ((def.shift ?? false) !== e.shiftKey) continue;
    if ((def.alt ?? false) !== e.altKey) continue;
    if (e.key.toLowerCase() === def.key) return cmdId;
  }
  return null;
}
