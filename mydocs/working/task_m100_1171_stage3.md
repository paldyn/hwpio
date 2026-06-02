# Stage 3 완료보고서 — Task #1171

- **이슈**: [#1171](https://github.com/edwardkim/rhwp/issues/1171)
- **브랜치**: `local/task1171`
- **단계 목표**: 프런트엔드 picture 우선 hit-test — 글상자 내부 클릭이 텍스트 편집으로
  단락되기 전에 글상자 안 picture 를 선제 선택.
- **작성일**: 2026-06-02

## 변경 내용 (`rhwp-studio/src/engine/input-handler-mouse.ts`)

글상자 경계선 검사(705-720) 직후, 텍스트 편집 진입(`if (hit.isTextBox)` 캐럿 배치) 직전에
선제 hit-test 분기 추가:
- `hit.isTextBox` 일 때 `findPictureAtClick(pageIdx, pageX, pageY)` 선제 호출.
- 반환 picHit 이 `type==='image'|'equation'` 이고 `cellPath`(글상자/셀 sentinel) 동반이면
  → 기존 picture dispatch(`enterPictureObjectSelectionDirect`, 842-848 호출과 동일 인자)로
  객체선택 진입 + `return`.
- picHit 없거나 picture 아니거나 cellPath 없으면 → 아래 기존 텍스트 편집으로 fall-through
  (picture 없는 글상자 영역 클릭은 기존대로 텍스트 편집).

설계 근거: 표 셀 picture 는 `hit_test_native` 가 `isTextBox=false` 라 기존 picture 처리
(762행)까지 fall-through 되지만, Shape text_box 는 `isTextBox=true` 라 744행 텍스트 편집에서
단락되어 762행에 도달하지 못했다. 본 선제 분기가 그 단락 이전에 글상자 picture 를 가로챈다.

## 검증

- `npx tsc --noEmit`: **변경 파일(input-handler-mouse.ts) 에러 0**. (전체 에러는
  canvaskit-renderer.ts 의 `canvaskit-wasm` 모듈 미설치 — 본 변경과 무관한 기존 문제.)
- 디스패치 인자는 기존 picture 선택 경로(842-848)와 동일하므로 select 상태/렌더 정합.
- **행위 검증(클릭→객체선택)은 Stage 5 통합 검증에서 수행** — findPictureAtClick 이
  Stage 1-2 의 Rust 변경(cellPath 노출)을 포함한 새 WASM 빌드를 필요로 한다(현재 pkg/ 는
  Stage 1-2 미반영). Stage 5 에서 WASM 재빌드 후 E2E/수동 검증.

## Stage 4 사전 분석 (insert.ts)

`insert.ts:272-283` 의 cellPath 재구성은 스칼라(`outerTableControlIdx`/`cellIdx`/`cellParaIdx`)
로 depth-1 경로 `[{controlIdx, cellIdx, cellParaIdx}]` 를 만들며, 키 이름이 백엔드
`parse_cell_path_json` 규약(`controlIdx`/`cellIdx`/`cellParaIdx`)과 일치한다. 글상자 picture 는
depth-1 이고 Stage 1 의 sentinel 로 세 스칼라가 모두 (0,0,0) 으로 채워지므로, **기존 재구성이
글상자 picture 에도 그대로 올바른 cellPath 를 생성할 가능성이 높다.** Stage 4 에서 WASM 빌드
후 실측하여, 기존 코드로 충분하면 무변경(또는 ref.cellPath 우선 + fallback 의 견고화만),
부족하면 보정한다.

## 다음 단계 (Stage 4 / 5)

- Stage 4: WASM 빌드 후 글상자 picture 속성 dialog read/write 실측 → insert.ts 보정 필요성 판단.
- Stage 5: 통합 검증 + tac-img-02.hwp p6/p7 수동 클릭(객체선택/속성/외곽 Shape 선택/텍스트 편집).
