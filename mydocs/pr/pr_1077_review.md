# PR #1077 검토 — raw_ctrl_data 오프셋 4바이트 밀림 (표 위치 편집 시 flags 오염)

## 1. PR 정보

| 항목 | 값 |
|------|-----|
| 번호 | #1077 |
| 제목 | fix: 표 위치 편집 시 raw_ctrl_data 오프셋 4바이트 밀림 — flags 오염 (#698) |
| 작성자 | oksure (Hyunwoo Park) — 기존 컨트리뷰터 |
| base ← head | `devel` ← `contrib/fix-table-position-offset` |
| 연결 이슈 | Closes #698 (assignee 본인 지정 완료) |
| mergeable | MERGEABLE / BEHIND (cherry-pick 으로 해소) |
| CI | Build & Test ✅ / Analyze ✅ / CodeQL ✅ |
| 커밋 | 3 (fix + 회귀 가드 test + cargo fmt) |
| 변경 | `src/document_core/commands/table_ops.rs` 단일 Rust 파일 |

## 2. 배경 (이슈 #698)

rhwp 로 생성한 표가 들어간 문서를 저장 후 다시 열면 표 위치가
줄 끝으로 밀려 보이지 않거나 위치 편집 불가. 보고된 환경:
firefox/chrome/edge/HOP 전부 동일 (v0.7.9).

## 3. 한컴 스펙 교차 검증 (작업지시자 지시)

### 3.1 스펙 — 표 69 (개체 공통 속성)

`mydocs/tech/한글문서파일형식_5.0_revision1.3.md:1442`:

| 자료형 | 길이 | 설명 |
|--------|------|------|
| UINT32 | 4 | **ctrl ID** |
| UINT32 | 4 | **속성(flags)** (표 70 참조) |
| HWPUNIT | 4 | **세로 오프셋** |
| HWPUNIT | 4 | **가로 오프셋** |
| HWPUNIT | 4 | width |
| HWPUNIT | 4 | height |
| ... | ... | ... |

스펙 절대 오프셋: ctrl_id [0..4], flags [4..8], v_offset [8..12],
h_offset [12..16], width [16..20], height [20..24].

### 3.2 rhwp `raw_ctrl_data` 의미 (실제 동작)

`src/parser/control.rs:153`: `table.raw_ctrl_data = ctrl_data.to_vec()`
— **ctrl_id 를 제외한 ctrl_data 본체** 저장.
`parse_common_obj_attr(ctrl_data)` (`shape.rs:332`): 첫 4 바이트를
`attr` 로 읽음.

→ **rhwp `raw_ctrl_data` 권위 레이아웃** (스펙 - ctrl_id):
- **[0..4] = attr (flags)**
- **[4..8] = vertical_offset**
- **[8..12] = horizontal_offset**
- **[12..16] = width**
- **[16..20] = height**

### 3.3 권위 자료 일치 확인

`src/model/table.rs:251` 주석 (rhwp 자체 명세):
> raw_ctrl_data 레이아웃 (attr 4바이트 이후):
>   [0..4] vertical_offset, [4..8] horizontal_offset,
>   [8..12] width, [12..16] height

→ attr(4) 이후 기준이므로 **전체로는 v_offset=[4..8], h_offset=[8..12]**
— 위 §3.2 와 정확히 일치.

`src/document_core/html_table_import.rs:495-509` (기존 HTML 표 import):
- `raw_ctrl_data[8..12]` = total_width (= attr 이후 [4..8])
- `raw_ctrl_data[12..16]` = total_height
→ 동일 레이아웃 일관 사용.

`src/document_core/converters/common_obj_attr_writer.rs`: HWPX→HWP
변환기. 같은 권위 자료 참조.

**결론**: rhwp 코드베이스 전체가 `raw_ctrl_data` 레이아웃에 대해
일관된 권위 자료([4..8]=v_offset, [8..12]=h_offset)를 사용. PR
**이전** `table_ops.rs` 만 4바이트 밀려 있었던 회귀였음.

## 4. PR 변경 분석

### 4.1 코드 수정 (`table_ops.rs`, 6곳)

| 위치 | Before | After | 정합 |
|------|--------|-------|------|
| `move_table_position` v_offset read | `[0..4]` | `[4..8]` | ✅ |
| `move_table_position` v_offset write | `[0..4]` | `[4..8]` | ✅ |
| `move_table_position` 문단 교환 후 v_offset | `[0..4]` | `[4..8]` | ✅ |
| `move_table_position` h_offset read | `[4..8]` | `[8..12]` | ✅ |
| `move_table_position` h_offset write | `[4..8]` | `[8..12]` | ✅ |
| `set_table_properties_json` v/h offset write | `+0`, `+4` | `+4`, `+8` | ✅ |
| `get_table_properties_json` v/h offset read | `+0`, `+4` | `+4`, `+8` | ✅ |
| 패딩 가드 | `len < 8` | `len < 12` | ✅ (h_offset 포함) |

기존 코드가 **flags 위에 v_offset 을 덮어쓰던 오염** — 정확히
첨부 이슈 사례 (`horz=문단(41950=148.0mm)` 비정상값 = flags 오염
결과) 와 부합.

### 4.2 회귀 가드 테스트 (커밋 f46fab29)

```rust
#[test]
fn raw_ctrl_data_offsets_match_parser() {
    // CommonObjAttr layout: [0..4]=flags, [4..8]=v_offset, [8..12]=h_offset
    let mut data = vec![0u8; 36];
    data[0..4].copy_from_slice(&flags.to_le_bytes());
    data[4..8].copy_from_slice(&42_u32.to_le_bytes());  // v_offset
    data[8..12].copy_from_slice(&99_u32.to_le_bytes()); // h_offset
    ...
    let common = parse_common_obj_attr(&data);
    assert_eq!(common.vertical_offset, 42);   // [4..8] 정합
    assert_eq!(common.horizontal_offset, 99); // [8..12] 정합
}
```

**`parse_common_obj_attr` 와 같은 진실 출처를 assert** — 향후
인덱스 회귀를 자동 검출. PR #1076 의 회귀 가드 부재 문제를 본
PR 은 자체 해소.

## 5. 검토 의견

### 5.1 강점

- **root cause 정확**: 4바이트 밀림으로 flags 가 v_offset 으로
  덮어써지고, v/h 가 뒤바뀌고, h_offset 이 미갱신되던 정확한
  진단. 이슈 첨부 dump 의 비정상 값 `horz=148mm` 와 부합.
- **rhwp 전체 코드베이스 권위 자료와 일치**: model/table.rs:251
  주석, html_table_import.rs, parse_common_obj_attr 모두 동일
  레이아웃 사용. **`table_ops.rs` 만 회귀였던 외톨이 코드**.
- **한컴 스펙 표 69 와도 정합** (스펙 - ctrl_id = rhwp raw_ctrl_data).
- **회귀 가드 테스트 동반** (PR #1076 부재 보완): 코드와 테스트가
  같은 진실 출처(`parse_common_obj_attr`) 를 검증 — 미래 회귀 방지.
- 주석에 정확한 레이아웃 명시 — 추후 유지보수 용이.
- 패딩 가드 `len < 8 → < 12` 도 함께 갱신 — h_offset 까지 보장.

### 5.2 검토 포인트

- **본질이 한컴 호환 파일 무결성** — `feedback_self_verification_not_hancom`:
  자체 cargo test 통과 외에 **한컴 수동 검증 게이트 권고**.
  이슈 #698 재현(표 이동 → 저장 → 한컴 재오픈 → 표 위치 정상)
  으로 검증 필요.
- **자매 PR 들과의 관계**:
  - PR #1078 (HTML 테이블 붙여넣기 raw_ctrl_data, #698 관련)
  - PR #1081 (CommonObjAttr 상수 모듈 리팩터링, #698 후속)

  → 본 PR 이 가장 직접적인 버그 fix. 자매 PR 들은 같은 영역의
    유사 회귀(#1078) 또는 상수화 리팩터링(#1081). 본 PR 먼저
    처리 후 자매 PR 검토 권고.

## 6. 검증 계획

- [ ] cherry-pick (`927b1003` + `f46fab29` + `7a8781e5` → 최신 devel)
- [ ] 전체 `cargo test` + `cargo clippy -D warnings` + `cargo fmt --check`
- [ ] WASM 빌드 (table_ops 변경 — WASM 영향)
- [ ] **한컴 수동 검증 게이트** (작업지시자 판단):
      이슈 #698 재현 시나리오 — 표 이동/속성 변경 → 저장 → 한컴
      재오픈 → 표 위치/속성 정상 확인

## 7. 판단 (잠정)

root cause 진단 정밀 + rhwp 전체 권위 자료와 정합 + 한컴 스펙
표 69 와 일관 + **회귀 가드 테스트 자체 동반** (PR #1076 보완).
`table_ops.rs` 가 같은 코드베이스 내 다른 모듈들과 다르게 4바이트
밀려 있던 명백한 회귀. 본 PR 은 그 외톨이 회귀를 정정.

본질이 한컴 호환 파일 무결성 → **한컴 수동 검증**이 판정 핵심.
검증 + 한컴 수동 검증 통과 시 수용 권고. 자매 PR #1078/#1081 은
본 PR 처리 후 별도 검토.

검증 결과에 따라 `pr_1077_report.md` 작성.
