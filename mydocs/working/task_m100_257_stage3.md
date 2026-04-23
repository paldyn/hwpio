# 단계3 완료 보고서: · 중점 시각적 중앙 배치 (A안)

- **타스크**: [#257](https://github.com/edwardkim/rhwp/issues/257)
- **마일스톤**: M100
- **브랜치**: `local/task257`
- **작성일**: 2026-04-23
- **단계**: 3 / 4

## 1. 배경

단계 2 (폴백 경로 narrow glyph base_w 분기) 완료 후, 작업지시자 피드백으로 `text-align-2.svg` 의 등록 폰트 경로(휴먼명조) 본문 `·` 도 narrow 로 수렴해 달라는 요청이 있었다. 단계 3 진입점에서 `measure_char_width_embedded` 의 is_halfwidth_punct 목록에서 `·` 를 `is_narrow_punctuation` 으로 이관 + 0.3 em 상한 캡을 먼저 시도했다 (초안 구현).

그 결과 본문 `세대별·지역별` · `시·청각장애인의` 의 `·` advance 가 8.40 → 5.37 px 로 축소됐으나, 작업지시자 재검증에서 **`·` 글리프가 우측(지·청) 이웃에 쏠려 보인다** 는 지적.

측정 결과 `·` 글리프 좌 여백 3.40 px > 우 여백 1.37 px 로 실제 "오른쪽 쏠림" 확인. 원인은 advance 자체가 문제 아니라 **글리프가 advance box 왼쪽에 자연 정렬** 되어 중앙 배치되지 않는 것. 한컴 PDF 는 advance=em/2 + 글리프 중앙 배치 조합.

## 2. 방침 전환 (A안)

초안 방향(embedded 경로 advance 축소) 을 revert 하고, **시각적 중앙 배치** 로 재설계.

| 항목 | 초안(revert 대상) | A안(채택) |
|-----|------------------|----------|
| `measure_char_width_embedded` | `·` 를 is_narrow_punctuation 로 이관, 0.3 em 캡 | 기존 로직 복원 (em/2 강제) |
| `·` advance (휴먼명조 20pt) | 5.37 px (0.27 × font_size) | 8.40 px (em/2 복귀) |
| SVG 렌더링 | `<text x=".">·</text>` (글리프 좌 정렬) | **`<text x=" + shift">·</text>` (글리프 중앙)** |
| Task #229 단조성 안전장치 | 동일 | 동일 |

## 3. 수행 내용

### 3.1 `measure_char_width_embedded` revert

`src/renderer/layout/text_measurement.rs:747-762` 을 단계 2 이전 상태로 복원:

```rust
let is_halfwidth_punct = matches!(c,
    '\u{2018}'..='\u{2027}' | '\u{00B7}'  // `·` 다시 포함
);
if is_halfwidth_punct && glyph_w >= mm.metric.em_size {
    mm.metric.em_size / 2
} else {
    glyph_w
}
```

- `·` 는 등록 폰트(휴먼명조)에서 em/2 로 강제되는 기존 로직 복원
- 단계 3 초안에서 추가한 `test_narrow_glyph_middle_dot_embedded_font` 테스트 제거
- 단계 2 의 `is_narrow_punctuation` 헬퍼는 **그대로 유지** (폴백 경로에서만 작동)

### 3.2 SVG 렌더러 중앙 배치 shift (신규)

`src/renderer/svg.rs:1794-1840` `draw_text` 에 중앙 배치 로직 추가:

```rust
let compute_center_shift = |cluster_str: &str, advance: f64| -> f64 {
    if cluster_str == "\u{00B7}" {
        let glyph_w_est = font_size * 0.15;
        ((advance - glyph_w_est) / 2.0).max(0.0)
    } else {
        0.0
    }
};
let cluster_advance = |char_idx: usize, cluster_str: &str| -> f64 {
    let n = cluster_str.chars().count();
    char_positions[char_idx + n] - char_positions[char_idx]
};
```

**적용 대상**: 현재는 `·` (U+00B7 MIDDLE DOT) 한 글자 한정.

**적용 원리**:
- `char_positions` 로 advance box 의 시작·끝 좌표 획득
- `·` 글리프 폭은 `font_size * 0.15` (narrow 글리프 관례) 추정
- shift = `(advance - glyph_w) / 2` → advance box 중앙에 글리프 배치
- 그림자 렌더링 루프·본문 렌더링 루프 양쪽 모두 적용

**적용 범위 제한**: `·` 만 처리. `,` `.` `:` 등은 baseline 에 붙는 글리프(comma 형태) 이므로 중앙 배치 대상 아님.

### 3.3 수치 비교 (휴먼명조 20pt 본문 `별·지`)

A안 초안 적용 후 작업지시자 재검증에서 "여전히 쏠려 보임" 지적. 분석 결과 **이전 char 의 trailing bearing** 을 고려하지 않아 시각적 불균형 잔존.

**공식 개선**:

- 초안: `shift = (advance - glyph_w) / 2`  → `·` advance box 안에서 중앙, 하지만 이전 char 의 오른쪽 여백이 advance 에 포함되어 시각상 우측 쏠림
- 최종: `shift = (advance - prev_trailing_bearing - glyph_w) / 2`  (prev_trailing_bearing ≈ prev_advance × 0.08, CJK 관례)

| 지표 | 초안(A안) | 최종(A안 refined) |
|-----|----------|----------------|
| · x 좌표 (SVG) | 173.44 | **172.71** |
| shift | 2.70 px | **1.97 px** |
| 좌 gap (별 글리프 끝 → · 글리프 시작) | 4.10 | **3.37** |
| 우 gap (· 글리프 끝 → 지 글리프 시작) | 2.70 | **3.43** |
| 좌/우 gap 비 | 1.52 | **0.98 (거의 완벽)** |

단, prev char 가 없는 문단 시작 `·` 나 ASCII 맥락 `a·b` 등은 trailing bearing 추정이 다를 수 있으나, 주 유스케이스(CJK 맥락 본문 `·`) 는 정확히 수렴.

### 3.4 HY중고딕 표 셀 (폴백 경로) 에서도 같은 개선 적용

단계 2 폴백 narrow 경로(`·` advance = 0.3 em ≈ 4.33 px) 에서도 shift 작동:
- shift = (4.33 - 16.67 × 0.15) / 2 = **0.92 px**
- advance 가 좁아 shift 작지만 대칭 정렬 유지

### 3.5 검증

| 검증 | 결과 |
|------|------|
| `cargo test --lib text_measurement::` | **22 pass / 0 fail** |
| `cargo test --lib renderer::` | **285 pass / 0 fail** |
| `cargo test --test svg_snapshot` (golden 재생성 후) | **3 pass** |
| `cargo clippy --lib -- -D warnings` | **통과** |

Task #229 회귀 테스트 4건도 baseline 그대로 통과 (advance 계산 로직은 단계 3 에서 변경 없음, `·` 에만 렌더링 shift 적용).

svg_snapshot golden 은 form-002·table-text 양쪽 재생성 (narrow `·` 렌더링 shift 로 인한 x 좌표 변경 반영).

## 4. 잔여 과제 (단계 4 로 이관)

- `min_w` 클램프 우회 검토: **불필요** 로 최종 확정. 사유:
  - 등록 경로 advance 는 원래 em/2 복귀 (narrow cap 없음)
  - 폴백 경로 narrow cap(0.3 em)은 단계 2 에서 그대로 유지, Task #229 회귀 테스트 전부 통과
- 스모크 스위프 (narrow glyph 다수 샘플): 단계 4
- 최종 결과보고서: 단계 4

## 5. 산출물

- `src/renderer/layout/text_measurement.rs`:
  - `measure_char_width_embedded` 원복 (`·` → is_halfwidth_punct)
  - 단계 3 초안 테스트 `test_narrow_glyph_middle_dot_embedded_font` 제거
- `src/renderer/svg.rs`:
  - `draw_text` 에 `·` 중앙 배치 shift 추가
- `tests/golden_svg/form-002/page-0.svg`, `tests/golden_svg/table-text/page-0.svg`: 재생성
- `output/svg/text-align-2/text-align-2.svg` (수정 후)
- `output/re/text-align-2-stage3-final-task257.svg` (참조 보관)
- `mydocs/working/task_m100_257_stage3.md` (본 보고서)

## 6. 요청 사항

A안으로 단계 3 완료. 승인 시 단계 4 (통합 검증 + 최종 보고서) 진행.
