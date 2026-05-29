# Task #1151 최종 결과 보고서 (v6 통합) — 표 + picture 한컴 정합

이슈: [#1151](https://github.com/edwardkim/rhwp/issues/1151)
브랜치: `local/task1151` → upstream `edwardkim:devel` PR

v1 보고서 (1차): [task_m100_1151_report.md](task_m100_1151_report.md) — 셀 안 floating picture 삽입만 정합
본 v6 통합 보고서는 v1 + v2 + v3 + v4 + v5 + v6 의 누적 산출물 전체를 정리한다.

## 1. 사용자 의도 (원 task scope)

> 표 + picture 시나리오의 한컴 정합 — 삽입 + 토글 + 시각 + 클릭

본 task 는 6 phase 누적으로 진행:

| Phase | 범위 | 산출 |
|------|------|------|
| v1 | 셀 안 picture 신규 삽입 (한컴 패턴: floating sibling) | `insert_picture_native` 셀 분기 + WASM/TS bridge |
| v2 | tac false→true 토글 시 outer paragraph inline 마이그레이션 | `migrate_picture_floating_to_inline` helper + 6 단위 + 4 통합 테스트 |
| v3 | `wrap=TopAndBottom` 표 + sibling inline picture 의 시각 layout 정합 | `calc_sibling_topandbottom_table_reserved_hu` helper + 6 단위 테스트 |
| v4 | 셀 안 inline picture 의 click hit-test + 개체 속성 dialog | 8 caller 갱신 + ImageNode 확장 + cursor_rect 셀 분기 + by_path API |
| v5 | `set_picture_properties_native` 의 page tree cache invalidate 누락 | 1 줄 fix + 1 regression 테스트 |
| v6 | `Table::update_ctrl_dimensions` 의 `self.common` 동기화 누락 | 2 줄 fix + 2 regression 테스트 |

## 2. v1 ~ v4 요약

### v1 (셀 floating picture 삽입)
- Fix: `insert_picture_native` 가 표 sibling 의 같은 paragraph 에 floating picture (tac=false, wrap=Square, Paper-relative offset) 를 control 로 추가.
- 자료: [Stage 1](../working/task_m100_1151_stage1.md) · [Stage 2](../working/task_m100_1151_stage2.md) · [Stage 3](../working/task_m100_1151_stage3.md).

### v2 (floating → inline 토글 model 정합)
- 한컴 정합 (Scenario A~D dump 비교): 4 필드만 갱신 — `treat_as_char`, `h/v_rel_to=Para`, `h/v_offset=0`, `parent.line_segs[0].line_height = picture_height`.
- Fix: `migrate_picture_floating_to_inline` helper + `set_picture_properties_native` 의 tac false→true migration 분기.
- 자료: [hancom_picture_tac_toggle.md](../tech/hancom_picture_tac_toggle.md) · [v2 Stage 1](../working/task_m100_1151_v2_stage1.md) · [Stage 2](../working/task_m100_1151_v2_stage2.md).

### v3 (TopAndBottom 표 + sibling inline picture 시각 layout)
- Fix: `calc_sibling_topandbottom_table_reserved_hu` helper — paragraph 내 sibling TopAndBottom 표의 outer_margin top + height + bottom 합산 → inline picture 의 y_baseline 을 그만큼 아래로 밀어 표 아래 그림.
- 자료: [Stage 3](../working/task_m100_1151_v3_stage3.md) · [Stage 4](../working/task_m100_1151_v3_stage4.md).

### v4 (셀 안 inline picture click + 개체 속성)
- Root cause (5-layer fault): ImageNode struct 미확장 + layout caller 8 곳 미전달 + cursor_rect skip + rendering.rs JSON 누락 + dialog cellPath 분기 부재.
- Fix: ImageNode 확장 + 8 caller 갱신 + cursor_rect 셀 분기 + JSON 직렬화 확장 + by_path API + dialog 분기.
- 진짜 진입점 확정: `picture_footnote.rs:57` `layout_picture_full`.
- 자료: [Stage 6](../working/task_m100_1151_v4_stage6.md) · [Stage 7](../working/task_m100_1151_v4_stage7.md).
- 검증: tac-img-02.hwp 7 페이지 표 셀 안 picture 모두 click ✓ (6/7 페이지 사각형 글상자 안 picture 는 #1171 별도).

## 3. v5 — page tree cache invalidate 누락 (사용자 추가 발견)

### 증상
v4 PR 직전 사용자가 rhwp-studio 시연 중 발견: 신규 표 + 셀 안 picture 삽입 + tac toggle 시 **시각 변화 없음**.

### 진단 (4 layer 검증)
| Layer | 상태 |
|-------|------|
| model (v1 + tac toggle) | ✓ 정상 (단위 테스트 PASS) |
| composer | ✓ 정상 (tac_controls 수집, ComposedLine.line_height = picture_height) |
| paragraph_layout / render tree | ✓ 정상 (build_page_render_tree → ImageNode 표 아래 정확 위치) |
| **page tree cache** | ✗ **stale** |

→ studio 가 `build_page_tree_cached` 로 stale cache 반환. Rust 측 layout 까지 모두 정합이나 cache 갱신 누락.

### Root cause
`src/document_core/commands/object_ops.rs:292` `set_picture_properties_native` 가 `paginate_if_needed()` 직후 **`invalidate_page_tree_cache()` 호출 누락**. 다른 6 개 picture/shape setter (셀 shape by_path / 셀 picture by_path / header-footer / shape 등) 모두 호출하지만 본 본문 picture setter 만 일관성 누락.

### Fix
1 줄 추가: `self.invalidate_page_tree_cache();`

### Regression test
`v5_tac_toggle_invalidates_page_tree_and_emits_inline_picture_below_table` — toggle 전후 `build_page_tree_cached` 결과의 ImageNode y 비교 + picture 위치 검증.

자료: [v5 Stage 9 보고서](../working/task_m100_1151_v5_stage9.md).

## 4. v6 — Table::update_ctrl_dimensions self.common 동기화 누락 (사용자 추가 발견)

### 증상
v5 fix 적용 + WASM 재빌드 후 사용자 재시연: 표 1×1 + **셀 크기 조절** + 셀 안 picture 삽입 + tac toggle 시 표 박스가 picture 영역까지 포함한 큰 박스 + picture 가 표 박스 안 좌상단 inline.

### 진단 (3 단계)
1. **한컴 정합 baseline (scenario-a-after.hwp)**: `table.common.height = 12498` HU = `cell[0].height` 동일. render tree 의 표 box height = cell.height. picture 가 표 아래 정확 배치.
2. **rhwp v1 + 셀 height 11216 delta 후**: `table.common.height = 1282` (resize 전 stale) ≠ `cell[0].height = 11498` (resize 적용). paragraph_layout 의 v3 helper 가 `t.common.height` 사용 → reservation 1848 HU (작음) → picture y 가 표 박스 안으로.
3. **Root cause**: `src/model/table.rs:255` `update_ctrl_dimensions` 가 `raw_ctrl_data` bytes 만 갱신하고 `self.common.width` / `self.common.height` 미동기화. `resize_table_cells_native` 가 `update_ctrl_dimensions` 호출하지만 `self.common` 은 stale.

### Fix
`update_ctrl_dimensions` 끝에 2 줄 추가:
```rust
self.common.width = total_width;
self.common.height = total_height;
```

### Regression tests
- `v6_render_tree_scenario_a_after_baseline` — 한컴 정합 model 의 render tree baseline 검증 (picture y > table bottom)
- `v6_resize_cell_then_tac_toggle_picture_below_table` — cell 조절 후 toggle → `table.common.height` 동기화 + render tree picture 위치 정합

### 사용자 시연 검증 통과 (2026-05-30)
> "정확함. rhwp에서 그렇게 만든 문서 한컴에서도 정합"

자료: [v6 Stage 10 보고서](../working/task_m100_1151_v6_stage10.md).

## 5. 회귀 검증 (전체 누적)

| 항목 | 결과 |
|------|------|
| `cargo test --lib` (1445 tests) | passed 1445 / failed 0 / ignored 6 |
| `cargo clippy --lib -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| v1 셀 안 picture 신규 삽입 시각 시연 | PASS |
| v2 통합 테스트 4 (한컴 산출물 양방향 정합) | PASS |
| v3 helper 단위 테스트 6 + 4 시각 시나리오 | PASS |
| v4 사용자 시연 (tac-img-02.hwp 7개 셀 picture) | PASS |
| v5 신규 regression 1 (cache invalidate) | PASS |
| v6 신규 regression 2 (table.common 동기화) | PASS |
| v6 사용자 시연 (한컴 정합 시각 + rhwp 생성 hwp 한컴 연 동등) | PASS |

## 6. 후속 task

- [#1171](https://github.com/edwardkim/rhwp/issues/1171) 사각형 글상자 안의 picture click hit-test (이중 nested). v4 진단 중 발견되어 별도 분리.

## 7. 변경 파일 (v1~v6 통합)

### Rust 모델
- `src/model/table.rs:255` — Table::update_ctrl_dimensions 에서 self.common 동기화 (v6)
- `src/renderer/render_tree.rs` — ImageNode struct + inline_shape_positions 정규화 (v4)

### Rust 렌더 layer
- `src/renderer/layout/picture_footnote.rs` — cell_ctx 시그니처 + set_inline_shape_position (v4)
- `src/renderer/layout/paragraph_layout.rs` — 3 곳 ImageNode + v3 sibling TopAndBottom helper (v3, v4)
- `src/renderer/layout/table_layout.rs` (v4)
- `src/renderer/layout/table_partial.rs` (v4)
- `src/renderer/layout/shape_layout.rs` (v4)
- `src/renderer/layout/table_cell_content.rs` (v4)
- `src/renderer/layout.rs` (v4)

### Rust API layer
- `src/document_core/commands/object_ops.rs`
  - v1: 셀 floating insert + v2: migrate helper + tac toggle 분기
  - v4: get/set_cell_picture_properties_by_path_native
  - v5: invalidate_page_tree_cache 호출 (1 줄)
- `src/document_core/queries/rendering.rs` — Image JSON 직렬화 확장 (v4)
- `src/document_core/queries/cursor_rect.rs` — 셀 안 inline shape hit-test 분기 (v4)
- `src/wasm_api.rs` — insert_picture + 셀 picture by_path export (v1, v4)

### Studio
- `rhwp-studio/src/core/wasm-bridge.ts` (v4)
- `rhwp-studio/src/ui/picture-props-dialog.ts` (v4)
- `rhwp-studio/src/command/commands/insert.ts` (v4)

### 테스트
- `issue_1151_v2_tac_toggle_tests` — 6 단위 (v2) + 4 통합 (v2) + 1 regression (v5) + 2 regression (v6) + 1 baseline (v6)
- `issue_1151_v3_sibling_topandbottom_tests` — 6 (v3)

### 자료
- `mydocs/tech/hancom_picture_tac_toggle.md` (v2)
- `mydocs/plans/task_m100_1151_{,v2,v3,v4}{,_impl}.md` (4 phase 계획서)
- `mydocs/working/task_m100_1151_{stage1, stage2, stage3, v2_stage1, v2_stage2, v3_stage3, v3_stage4, v4_stage6, v4_stage7, v5_stage9, v6_stage10}.md`

## 8. 결론

원 task scope ("표 + picture 한컴 정합 — 삽입 + 토글 + 시각 + 클릭") 의 4 측면 모두 완성. 추가로 사용자가 시연 중 발견한 2 개 결함 (v5 cache invalidate + v6 table.common 동기화) 도 same-PR 으로 fix. 1 개 별도 결함 (#1171) 만 후속 task 로 이관.

→ 통합 PR (devel) 발행 + Issue #1151 close.
