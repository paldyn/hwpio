# PR #1264 검토 - 문28 조건 박스와 미주 간격 정합 보정

- **작성일**: 2026-06-03
- **PR**: #1264
- **제목**: `task 1261: 문28 조건 박스와 미주 간격 정합 보정`
- **컨트리뷰터**: @jangster77
- **연결 이슈**: #1261
- **base/head**: `devel` <- `task_m100_1261`
- **PR head**: `4c42146e0f1f52f15938b60aec1b9368b43009ec`
- **PR 기준 base**: `143fb0f4`
- **현재 devel**: `09f6b8d1`
- **검증 브랜치**: `local/pr1264-current`
- **검증 HEAD**: `11e79cee`
- **규모**: 15 files, +665 / -36
- **GitHub mergeable**: true
- **PR 댓글**: 없음

## 1. PR 요약

PR #1264는 두 계열 문제를 함께 보정한다.

1. `samples/3-10월_교육_통합_2022.hwp` 5쪽 문28 조건 박스:
   - TAC Shape의 실제 높이가 `common.height`보다 `shape_attr.current_height`에 더 가깝게 저장된 케이스에서,
     조건 박스 높이가 작게 잡혀 선택지 줄과 겹치는 문제를 보정한다.
2. `samples/3-09월_교육_통합_2024-미주사이20.hwp` 10쪽 문8/문12:
   - 직전 미주 수식의 저장 vpos trailing을 실제 콘텐츠 하단으로 오인해 `미주 사이 20mm`가 사라지거나,
     반대로 page-path vpos에서 gap이 중복 적용되어 문12 꼬리가 단 하단을 넘는 문제를 보정한다.

핵심 변경은 다음이다.

- TAC Shape 높이 계산 시 `common.height`와 `shape_attr.current_height` 중 큰 값을 사용한다.
- 렌더러가 직전 항목의 실제 콘텐츠 하단을 `HeightCursor.prev_item_content_bottom_y`로 전달한다.
- compact 미주 문항 제목의 위치를 실제 보이는 콘텐츠 하단 + `endnote_between_notes_hu` 기준으로 산출한다.
- 단일 줄 prev의 injected between-notes 경계에서 page/lazy vpos base를 signed delta로 보정해 후속 항목 누적 밀림을 막는다.

## 2. 주요 변경 범위

| 파일 | 변경 |
|---|---|
| `src/renderer/layout/paragraph_layout.rs` | TAC Shape 높이에 `shape_attr.current_height` 반영, line별 TAC shape 판정 보정 |
| `src/renderer/layout.rs` | 직전 항목 실제 콘텐츠 하단을 `HeightCursor`에 전달 |
| `src/renderer/height_cursor.rs` | compact 미주 제목 gap 산출과 page/lazy base signed 보정 |
| `src/renderer/typeset.rs` | typeset 경로의 `prev_item_content_bottom_y` 기본값 설정 |
| `tests/issue_1139_inline_picture_duplicate.rs` | 문28/문8/문12 회귀 테스트 추가 |
| `pdf-large/*.pdf` | 한컴 PDF bbox 비교 기준 추가 |
| `mydocs/orders/20260603.md` | 작업 기록 |
| `mydocs/*/task_m100_1261*.md` | 계획/작업/보고 문서 추가 |

## 3. 타당한 부분

### 3.1 TAC Shape 높이 보정은 문28 원인에 직접 대응한다

문28 조건 박스는 글자처럼 취급되는 Shape이고, 기존 로직은 `common.height`만 높이 예약에 사용했다.
PR은 Shape의 `current_height`까지 함께 고려해 실제 한컴 렌더링에 가까운 높이를 확보한다.

특히 다음 로직이 line별 TAC offset을 기준으로 동작하도록 좁혀져, 다른 줄의 TAC 컨트롤 때문에
현재 줄을 과하게 보정하는 위험을 줄였다.

```text
has_tac_shape = line_tac_offsets 기준
shape_h = max(common.height, shape_attr.current_height)
```

### 3.2 미주 간격 보정은 "저장 trailing"과 "실제 콘텐츠 하단"을 분리한다

문8 케이스는 직전 수식 줄의 저장 vpos/line spacing을 실제 보이는 수식 하단처럼 취급하면서
20mm 미주 사이 gap이 사라지는 문제다. PR은 렌더러가 측정한 `last_item_content_bottom`을
`HeightCursor`에 전달해, 실제 보이는 하단을 기준으로 gap을 더한다.

typeset 경로는 `prev_item_content_bottom_y=None`으로 남겨 기존 height-only 계산의 영향 범위를 제한한다.

### 3.3 문12 누적 밀림 보정은 base desync를 같이 처리한다

단순히 `y_offset`만 반환하면 후속 absolute vpos 기준과 렌더 위치가 다시 어긋날 수 있다.
PR은 `result - y_offset` signed delta를 page/lazy base에 반영해 후속 항목이 같은 기준을 따르게 한다.

이는 #1256/#1261 계열에서 반복된 "보이는 위치와 vpos base의 불일치" 문제를 줄이는 방향이다.

## 4. 위험 및 주의 사항

### 4.1 `HeightCursor`에 렌더러 측정값이 추가된다

`prev_item_content_bottom_y`는 `HeightCursor`가 pure vpos/line_seg 정보만 보던 구조에서 한 단계 더
렌더러 상태를 참조하게 만드는 변경이다. 현재는 compact 미주 + tall inline item + question title 분기로
좁혀져 있지만, 장기적으로는 미주 sequential flow SSOT 정리 대상에 포함해야 한다.

완화 요인:

- `is_finite()` 필터를 거친다.
- typeset 경로는 `None`으로 유지한다.
- 신규 단위 테스트가 실제 콘텐츠 하단 gap과 20mm gap을 직접 검증한다.

### 4.2 signed base shift는 민감한 변경이다

기존 `suppressed_hu`는 양수 방향만 반영했지만, 이번 PR은 `base_delta_hu`를 signed로 반영한다.
문12 중복 gap을 막기 위해 필요하지만, 미주 absolute-vpos 경계 전체에 영향 가능성이 있으므로
기존 #1189/#1209/#1256 후보의 시각 판정이 필요하다.

### 4.3 PR base가 archive 정리 이전이다

PR 기준 base는 `143fb0f4`이고 현재 `devel`은 `09f6b8d1`이다. 현재 `mydocs/plans`,
`mydocs/report`, `mydocs/working`의 즉시 하위 문서는 archive로 이동하는 정책이 적용되었다.

따라서 검증 브랜치에서는 PR 문서를 다음처럼 이동해 통합 후보로 정리했다.

| 원래 경로 | 통합 후보 경로 |
|---|---|
| `mydocs/plans/task_m100_1261*.md` | `mydocs/plans/archives/` |
| `mydocs/report/task_m100_1261_report.md` | `mydocs/report/archives/` |
| `mydocs/working/task_m100_1261_stage*.md` | `mydocs/working/archives/` |

### 4.4 PDF 2개가 LFS pointer가 아닌 실제 blob으로 들어왔다

`pdf-large/*.pdf`는 `.gitattributes` 기준 LFS 대상이다. 체리픽 중 다음 경고가 발생했다.

```text
Encountered 2 files that should have been pointers, but weren't
```

현재 커밋 객체의 PDF 크기도 약 1.47MB 실제 blob으로 확인된다.

```text
pdf-large/3-09월_교육_통합_2024-구분선아래20-2024.pdf: 1,472,249 bytes
pdf-large/3-09월_교육_통합_2024-미주사이20-2024.pdf: 1,473,246 bytes
```

따라서 이 PR은 GitHub의 PR branch를 그대로 merge하지 말고, 메인테이너 통합 브랜치에서
문서 archive 이동과 PDF LFS normalization을 함께 적용한 커밋으로 반영하는 것을 권장한다.

## 5. 자동 검증 결과

현재 `devel` 위에 PR 커밋을 체리픽해 검증했다.

| 항목 | 명령 | 결과 |
|---|---|---|
| cherry-pick | `143fb0f4..local/pr1264-verify` -> `local/pr1264-current` | 통과, 충돌 없음 |
| whitespace | `git diff --check devel..HEAD` | 통과 |
| Rust fmt | `cargo fmt --all --check` | 통과 |
| height_cursor 단위 테스트 | `cargo test --lib height_cursor -- --nocapture` | 통과, 34 passed |
| 미주/페이지네이션 통합 테스트 | `cargo test --test issue_1139_inline_picture_duplicate -- --nocapture` | 통과, 46 passed |

통합 테스트 중 기존 overflow diagnostic 로그가 일부 출력되었지만 테스트 실패는 없다.

## 6. 시각 판정 권장 후보

| file | page | 확인 항목 |
|---|---:|---|
| `samples/3-10월_교육_통합_2022.hwp` | 5 | 문28 조건 박스 하단과 선택지 줄 겹침 여부 |
| `samples/3-09월_교육_통합_2024-미주사이20.hwp` | 10 | 문8 제목이 직전 수식 하단 뒤 20mm gap을 유지하는지 |
| `samples/3-09월_교육_통합_2024-미주사이20.hwp` | 10 | 문10/문11/문12 제목 위치와 문12 꼬리 단 하단 overflow 여부 |
| `samples/3-09월_교육_통합_2024-구분선아래20.hwp` | 10 | PDF reference가 함께 추가된 endnote gap guard |

판정 포인트:

- 문28 선택지가 조건 박스 안쪽 텍스트와 겹치지 않는지
- 문8 제목 위 간격이 한컴 PDF bbox와 유사한지
- 문12 꼬리가 10쪽 오른쪽 단 안에 남는지
- 기존 #1189/#1209/#1256 미주 간격이 새로 흔들리지 않는지

## 7. 권장 처리

권장안: **수용 후보로 진행하되, 그대로 GitHub merge하지 말고 메인테이너 통합 커밋으로 정리**한다.

근거:

- PR이 주장한 세 핵심 케이스를 자동 테스트로 직접 검증한다.
- 현재 `devel` 위에서 PR 커밋이 충돌 없이 적용되고, 핵심 테스트가 통과했다.
- TAC Shape 높이 보정과 미주 gap 보정 모두 원인에 직접 대응한다.
- 다만 archive 정리 이후 문서 경로와 LFS PDF pointer 문제가 있어, 통합 단계에서 정리가 필요하다.

## 8. 다음 승인 요청

권장 절차:

```text
1. 필요 시 PDF LFS normalization 포함한 통합 커밋 정리
2. WASM 빌드
3. 위 샘플 기준 SVG/rhwp-studio 메인테이너 시각 판정
4. 통과 시 devel 병합
5. devel 기준 테스트 재확인
6. origin/devel push
7. PR #1264 및 이슈 #1261 종료 처리
```
