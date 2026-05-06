# PR #611 검토 보고서

**PR**: [#611 fix: add undo history for non-text resize operations (closes #458)](https://github.com/edwardkim/rhwp/pull/611)
**작성자**: @kihyunnn (김기현, mable0927@gmail.com) — **첫 PR 컨트리뷰터**
**상태**: OPEN, **mergeable=MERGEABLE**, **mergeStateStatus=BEHIND** (PR base 283 commits 뒤 — 4/29 등록 후 본 사이클 누적)
**관련**: closes #458
**처리 결정**: ⏳ **검토 중** (1차 검토)
**검토 시작일**: 2026-05-06

## 1. 검토 핵심 질문

1. **본질 결함 식별 정합성** — rhwp-studio 의 표/그림 리사이즈 경로가 `CommandHistory` 를 통하지 않고 WASM mutation 직접 호출 → Ctrl+Z / Ctrl+Y 미동작. 본 PR 의 `ResizeObjectCommand` (그림/도형) + `executeOperation kind: 'snapshot'` (표) 도입으로 Undo/Redo 정합 회복?
2. **회귀 위험** — rhwp-studio TypeScript 영역 단독 변경 (Rust 영역 무관). 텍스트 undo 경로 / 표 이동 undo 경로 보존 정합?
3. **PR base skew (4/29 등록 → 5/6 현재 283 commits 뒤)** — 본 환경 cherry-pick 충돌 0?
4. **첫 PR 컨트리뷰터 영역** — `feedback_pr_comment_tone` (차분/사실 중심) 정합 운영

## 2. PR 정보

| 항목 | 값 | 평가 |
|------|-----|------|
| 제목 | fix: add undo history for non-text resize operations | 정합 (영문) |
| author | @kihyunnn (김기현, mable0927@gmail.com) — **첫 PR 컨트리뷰터** | 신규 컨트리뷰터 |
| changedFiles | **5** / +128 / -19 | rhwp-studio TypeScript 5 파일 (Rust 영역 0) |
| 본질 변경 | `command.ts` (+54, ResizeObjectCommand 신규) + `input-handler-table.ts` (+30/-12) + `input-handler-picture.ts` (+40/-7) + `input-handler-mouse.ts` (+2) + `input-handler.ts` (+2) | 단일 영역 (rhwp-studio editor) |
| **mergeable** | MERGEABLE (UI), **mergeStateStatus=BEHIND** (PR base 283 commits 뒤) | 본 환경 cherry-pick 충돌 0 (auto-merge picture.ts 만 발생) |
| Issue | closes #458 (4/29 등록, 외부 사용자 등록) | ✅ |
| Issue assignee | 미지정 (`feedback_assign_issue_before_work` 영역) | 외부 컨트리뷰터 자율 영역 |
| **PR 본문** | 변경 요약 + 범위 한정 + 테스트 체크 + WASM/npm build 검증 명시 | 완성도 높음, hyper-waterfall 절차 부재 (작은 본질로 영역 정합) |

## 3. PR 의 1 commit 분석

| commit | 설명 | 본 환경 처리 |
|--------|------|-------------|
| **`a4944d2b`** fix: add undo history for non-text resize operations | rhwp-studio TS 5 파일 +128/-19 | ⭐ cherry-pick |

→ **단일 본질 commit**. PR #607 와 동일 패턴 (작은 본질 + fork plans/working/report 부재).

## 4. 본질 변경 영역

### 4.1 결함 가설 (Issue #458 인용)

> 표 크기를 실수로 수정하고 되돌리기를 하면 표는 원래대로 돌아오는게 아닌 무조건 텍스트만 뒤로가기 가능. (...) 이것은 text를 제외한 나머지(이미지, 표, 기타등등) 모두 포함입니다. 오직 text 만 되돌리기가 됩니다.

→ rhwp-studio 의 표 리사이즈 + 그림/도형 리사이즈 경로가 `CommandHistory` 우회 (WASM mutation 직접 호출) → Ctrl+Z / Ctrl+Y 미동작.

### 4.2 정정 — `command.ts` 신규 `ResizeObjectCommand` (+54 LOC)

```typescript
export type ObjectResizeTarget = {
  sec: number;
  ppi: number;
  ci: number;
  type: string;
  before: Record<string, unknown>;
  after: Record<string, unknown>;
};

export class ResizeObjectCommand implements EditCommand {
  readonly type = 'resizeObject';
  // setProps, execute, undo: before/after 속성 set
  // mergeWith(): null { return null; }
}
```

**before/after snapshot 영역**: `width/height/horzOffset/vertOffset` 변경 영역 모두 보존, `setShapeProperties` (shape/line/group) / `setPictureProperties` (image) 분기 정합.

### 4.3 표 리사이즈 — `input-handler-table.ts` (+30/-12)

3 분기 (`finishResizeDrag` / `resizeCellByKeyboard` / `resizeTableProportional`) 모두 동일 패턴:

```typescript
// Before
this.wasm.resizeTableCells(...)
this.eventBus.emit('document-changed');

// After
this.executeOperation({
  kind: 'snapshot',
  operationType: 'resizeTableCells',  // 또는 resizeCellByKeyboard / resizeTableProportional
  operation: (wasm: any) => {
    wasm.resizeTableCells(...);
    return this.cursor.getPosition();
  },
});
```

→ **snapshot 기반 Undo** 패턴 활용 (기존 `executeOperation` API + `kind: 'snapshot'` 분기 — 표 리사이즈는 복합 셀 보상 변경으로 단일 속성 set 보다 snapshot 이 정합).

### 4.4 그림/도형 리사이즈 — `input-handler-picture.ts` (+40/-7)

**다중 선택 리사이즈 (`finishPictureResizeDrag` multi-resize)**:

```typescript
const historyTargets = [];
for (const r of state.multiRefs) {
  // ... 새 width/height/horzOffset/vertOffset 산출
  const updated: Record<string, unknown> = { width: newW, height: newH };
  const before: Record<string, unknown> = { width: r.origWidth, height: r.origHeight };
  if (deltaH !== 0) { updated['horzOffset'] = ...; before['horzOffset'] = r.origHorzOffset; }
  if (deltaV !== 0) { updated['vertOffset'] = ...; before['vertOffset'] = r.origVertOffset; }
  const changed = Object.keys(updated).some(key => updated[key] !== before[key]);
  if (!changed) continue;
  setObjectProperties.call(this, r, updated);
  historyTargets.push({ sec: r.sec, ppi: r.ppi, ci: r.ci, type: r.type, before, after: updated });
}
if (historyTargets.length > 0) {
  this.executeOperation({ kind: 'record', command: new ResizeObjectCommand(historyTargets) });
}
```

→ **changed 가드** (Object.keys 비교) 로 무변경 케이스 record 차단 + 다중 선택 모두 단일 ResizeObjectCommand 로 묶음 (Undo 1회로 다중 복원).

**단일 선택 리사이즈** 동일 패턴.

### 4.5 보조 변경 — `input-handler-mouse.ts` (+2) + `input-handler.ts` (+2)

`onClick` 의 PictureResizeState 초기화 시 `origHorzOffset` / `origVertOffset` 추가 + state interface type 에 `origHorzOffset?: number` / `origVertOffset?: number` 옵셔널 추가 — `ResizeObjectCommand` before snapshot 영역에 필요한 origin 값 보존.

### 4.6 회귀 위험 영역 점검

PR 본문 명시:
- 텍스트 undo 경로 변경 0 (`finishResizeDrag` 외 텍스트 입력 경로 무관)
- 표/개체 이동 undo 경로 보존 (이미 `MovePictureCommand`/`MoveShapeCommand` 통과 영역, 본 PR 변경 없음)
- 기존 `executeOperation` API 활용 — 신규 인프라 도입 없이 기존 패턴 재사용

→ **회귀 위험 영역 좁음** — `MoveShapeCommand` 직후 위치에 `ResizeObjectCommand` 추가 + 표 리사이즈 3 분기를 snapshot 기반으로 감쌈.

## 5. 본 환경 직접 검증 (임시 브랜치 `pr611-cherry-test`)

| 단계 | 결과 |
|------|------|
| `a4944d2b` cherry-pick | ✅ 충돌 0 (auto-merge picture.ts 만 발생, 깨끗 통과) |
| `cargo build --release` | ✅ Finished (Rust 영역 변경 없음, 사전 빌드 정합) |
| `cargo test --lib --release` | ✅ **1140 passed** / 0 failed (회귀 0) |
| `cargo clippy --release --lib` | ✅ 0건 |
| **rhwp-studio `npm run build`** (`tsc && vite build`) | ✅ **TypeScript 타입 체크 통과 + dist 빌드 성공** (`dist/index.html` 55KB / `index.js` 689KB / `wasm` 4.59MB) |

→ **본 환경 base skew 283 commits 영향 0** — Rust 영역 변경 없음 + TypeScript 영역 단독 변경, cherry-pick 깨끗 통과 + 빌드 정합.

## 6. 광범위 페이지네이션 회귀 sweep

| 통계 | 결과 |
|---|---|
| 총 fixture | **164** (158 hwp + 6 hwpx) |
| 총 페이지 (BEFORE) | **1,684** |
| 총 페이지 (AFTER) | **1,684** |
| **fixture 별 페이지 수 차이** | **0** |

→ rhwp-studio TypeScript 변경은 페이지네이션 (Rust 영역) 에 영향 없음 (자명).

## 7. PR 본문의 자기 검증 결과

| 검증 | PR 본문 결과 | 본 환경 재검증 |
|------|---------|----------|
| `cargo test` | 통과 | ✅ 1140 passed |
| `cargo clippy -- -D warnings` | 통과 | ✅ 0건 |
| Docker WASM 빌드 | 통과 | ⏳ 현재 baseline WASM 4,588,198 bytes 정합 (PR #607 측정 활용) |
| `cd rhwp-studio && npm run build` | 통과 | ✅ tsc + vite build 정합 |
| `git diff --check` | 통과 | ✅ 본 환경 정합 |
| 관련 샘플 파일로 SVG 내보내기 확인 | ☐ 미체크 (Rust 영역 무관) | 무관 — TypeScript editor 영역 |
| **메인테이너 시각 판정** | (미진행) | ⏳ 본 환경 시각 판정 게이트 (web 기반 Undo/Redo 동작 확인) |

## 8. 메인테이너 정합성 평가

### 정합 영역 — 우수
- ✅ **본질 진단 정확** — rhwp-studio editor 의 CommandHistory 우회 영역 (표 리사이즈 + 그림/도형 리사이즈) 정확 식별
- ✅ **단일 영역 본질 정정** — rhwp-studio TypeScript 5 파일 + 기존 `executeOperation` API 활용 (신규 인프라 0)
- ✅ **다중 선택 + 단일 선택 모두 정합** — `historyTargets` 배열로 Undo 1회 다중 복원
- ✅ **changed 가드** — 무변경 케이스 record 차단 (Undo 스택 오염 방지)
- ✅ **표 3 분기 동일 패턴** — `finishResizeDrag` / `resizeCellByKeyboard` / `resizeTableProportional` 모두 snapshot 기반으로 통일
- ✅ **결정적 검증 정합** — 1140 passed / clippy 0 / TypeScript build 정합 / 광범위 sweep 회귀 0
- ✅ **PR 본문 정합** — 변경 요약 + 범위 한정 ("텍스트 undo 경로 변경 0, 이동 undo 경로 보존") + 검증 체크 모두 명시
- ✅ **Issue #458 권위 영역** — 외부 사용자 사용성 결함 (Ctrl+Z 미동작) 정확히 해결
- ✅ **첫 PR 컨트리뷰터 영역** — 깔끔한 본질 + 작은 영역 + 기존 패턴 재사용

### 우려 영역
- ⚠️ **시각 판정 영역** — 본 PR 은 web 기반 Undo/Redo 동작 영역 → SVG byte 비교 무관, **메인테이너 직접 web 환경에서 Ctrl+Z / Ctrl+Y 검증 필요**
- ⚠️ **표 cell 분할/병합 등 다른 표 변경 영역** — 본 PR 은 리사이즈만 다룸 (셀 분할/병합/삽입/삭제 등은 별도 영역, 본 PR 범위 외 — Issue #458 의 "기타등등" 영역 일부)
- ⚠️ **PR base 283 commits 뒤** — UI MERGEABLE 표시지만 본 환경 cherry-pick 충돌 0 확인 (저위험 영역)

## 9. 1차 검토 결론

### 정합 평가
- ✅ **본질 cherry-pick 가능** — `a4944d2b` 단일 commit, 5 파일 충돌 0
- ✅ **결정적 검증** — 1140 passed / clippy 0 / TypeScript build 정합 / 광범위 sweep 회귀 0
- ✅ **본질 정합** — Issue #458 의 표/그림 리사이즈 Undo 미동작 영역 정확히 해결
- ✅ **회귀 위험 좁음** — 텍스트 undo 경로 무변경, 이동 undo 보존, 기존 `executeOperation` API 재사용
- ⏳ **메인테이너 web 환경 시각 판정** — Ctrl+Z / Ctrl+Y 동작 영역 (브라우저 기반)

### 권장 처리 방향 (작업지시자 결정 대기)

#### 옵션 A — 핀셋 cherry-pick + web 환경 시각 판정 (권장)
- `a4944d2b` 단일 commit cherry-pick (author kihyunnn 보존)
- 본 환경 결정적 재검증 + TypeScript build 정합
- **메인테이너 web 환경 시각 판정** — vite dev server 실행 + 브라우저에서 (1) 표 리사이즈 후 Ctrl+Z 정상 복원 (2) 그림/도형 리사이즈 후 Ctrl+Z 정상 복원 (3) Ctrl+Y 정상 복원 (4) 텍스트 undo 회귀 0 확인
- 통과 시 devel merge + push + PR close (한글 댓글)

#### 옵션 B — 추가 영역 요청
- Issue #458 "기타등등" 영역 (셀 분할/병합/삽입/삭제 등) 별도 후속 task 권유

#### 옵션 C — close + 본 환경 직접 처리

→ **작업지시자 결정 대기**. 옵션 A 권장.

## 10. 메모리 정합

- ✅ `feedback_essential_fix_regression_risk` — 광범위 페이지네이션 sweep (164 fixture / 1,684 페이지) + 1140 passed 회귀 0
- ✅ `feedback_hancom_compat_specific_over_general` — 단일 영역 (rhwp-studio editor 리사이즈) 본질 정정, 다른 영역 무영향
- ✅ `feedback_rule_not_heuristic` — 기존 `executeOperation` API 재사용, 휴리스틱 미도입
- ✅ `feedback_visual_regression_grows` — web 환경 시각 판정 게이트 정합 운영 (브라우저 기반 Undo/Redo 동작)
- ✅ `feedback_pdf_not_authoritative` — PDF 미사용 (web editor 영역)
- ✅ `feedback_per_task_pr_branch` — Task #458 단일 본질 PR 정합
- ✅ `feedback_pr_comment_tone` — 첫 PR 컨트리뷰터 환영 + 차분/사실 중심 톤
- ✅ `feedback_check_open_prs_first` — PR OPEN 상태 + 본 사이클 처리분 점검 후 진행
- ✅ `feedback_assign_issue_before_work` — Issue #458 assignee 미지정 (외부 사용자 등록 + 외부 컨트리뷰터 정정 자율 영역, 회귀 후속 issue 영역 점검 — 본 영역은 정합)
- ✅ `feedback_small_batch_release_strategy` — v0.7.10 후 처리분 영역
- ✅ **신규 컨트리뷰터 첫 PR 영역** — 차분/정중한 톤 + 본질 정합 인정 패턴
- ⚠️ **rhwp-studio TypeScript 영역** — 본 사이클 첫 web editor 본질 정정 PR (이전 사이클 PR 들은 모두 Rust 렌더러 영역)

---

**검토자**: 클로드 (페어 프로그래밍 파트너)
**1차 검토 단계 — 작업지시자 결정 대기**.
