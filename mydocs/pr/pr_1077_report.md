# PR #1077 처리 보고 — raw_ctrl_data 오프셋 4바이트 밀림 (부분 해소)

## 1. 결정

**수정 요청 (거절)** — 자동 검증 통과했으나 한컴 수동 검증에서
이슈 #698 증상 잔존. PR 수정 범위 외 동일 버그 발견.

| 항목 | 값 |
|------|-----|
| 번호 | #1077 |
| 작성자 | oksure (Hyunwoo Park) |
| 연결 이슈 | Closes #698 (증상 **잔존**) |
| 상태 | **OPEN 유지, 수정 요청** (PR 댓글 게시) |

## 2. 검증 결과

### 자동 검증 (통과)

| 항목 | 결과 |
|------|------|
| cherry-pick `65db20f2+a4514b17+fb74c54c` | ✅ 충돌 없음 |
| 회귀 가드 `raw_ctrl_data_offsets_match_parser` | ✅ 통과 |
| 전체 `cargo test` | ✅ 1587 passed, 0 failed |
| `cargo clippy -- -D warnings` | ✅ 0 warnings |
| `cargo fmt --all -- --check` | ✅ 위반 0건 |
| WASM 빌드 | ✅ 성공 |
| CI | ✅ 전부 pass |

### 한컴 수동 검증 — **실패**

작업지시자 시나리오:
1. rhwp-studio 표 생성
2. 전체 셀 선택 후 일괄 크기 조절
3. 저장 → `saved/tb-002.hwp`
4. 한컴 / rhwp-studio 재오픈 → `horz=문단(35952=126.8mm)` 비정상
5. 속성창에서 `horzOffset=0` 설정 → 저장 → 다시 망가짐

## 3. 결함 진단 (확정)

`saved/tb-002.hwp` raw_ctrl_data 분석:

| offset | 값 | 권위 의미 | 평가 |
|--------|-----|----------|------|
| [0..4] | 0x002a0310 | flags | ✅ |
| [4..8] | 0 | v_offset | ✅ |
| **[8..12]** | **35952 (126.8mm)** | h_offset | ❌ **width 값이 잘못 기록** |
| [12..16] | 847 (3.0mm) | width | ❌ height 값이 들어감 |
| [16..20] | 3846 (13.6mm) | height | ❌ z_order 값 위치에 height |

### Root cause — PR 수정 범위 밖 잔존 버그

**`src/model/table.rs:251` `update_ctrl_dimensions`**:
```rust
// 주석: "attr 4바이트 이후" — 잘못된 기준 설명
self.raw_ctrl_data[8..12].copy_from_slice(&total_width.to_le_bytes());
self.raw_ctrl_data[12..16].copy_from_slice(&total_height.to_le_bytes());
```

이 함수가 `resize_table_cells_native` (table_ops.rs:799) 에서
호출되어 표 크기 조절 시 **total_width 를 h_offset 슬롯([8..12]) 에,
total_height 를 width 슬롯([12..16]) 에 잘못 기록**.

권위 레이아웃 (PR 검토 문서와 동일):
- [0..4] flags / [4..8] v_offset / [8..12] h_offset
- **[12..16] width / [16..20] height**
- [20..24] z_order / [24..32] outer_margin × 4

### 추가 잔존 버그 의심 위치

PR 검토 중 발견한 같은 패턴 다른 위치:
1. `update_ctrl_dimensions` width/height 슬롯 (위 핵심)
2. `set_table_properties_native:1248-1262` outer_margin `[20..28]`
   → 권위는 `[24..32]`
3. `update_ctrl_dimensions` 주석 자체가 raw_ctrl_data 전체 기준
   인덱싱과 불일치

## 4. 처리

- **PR #1077 OPEN 유지** — 작성자에게 수정 요청 댓글 게시
  (https://github.com/edwardkim/rhwp/pull/1077#issuecomment-4524721648)
- cherry-pick 브랜치 `pr1077-cherry` 삭제
- 이슈 #698 OPEN 유지 (PR 재푸시 시 재검토)
- local/devel `dd342ac0` 으로 origin/devel 정렬 (그 사이 외부 push 통합)
- **자매 PR #1078** (HTML 테이블 raw_ctrl_data) 도 동일 영역 →
  본 PR 처리 후 별도 검토

## 5. 평가

### PR 의 강점 (수정 범위 내)
- `move_table_position`, `set/get_table_properties_json_native` 의
  v/h_offset 오프셋 수정은 정확.
- 회귀 가드 테스트 (`raw_ctrl_data_offsets_match_parser`) 동반 —
  PR #1076 보완.

### 한계
- **PR 가 #698 의 일부 경로만 해소** — \"표 위치 편집\" (move/
  속성창) 은 고쳤으나 \"표 크기 조절\" (resize_table_cells) 경로의
  동일 버그 (`update_ctrl_dimensions`) 누락.
- 회귀 가드도 v/h_offset 만 검증, width/height/outer_margin 미포함
  → 잔존 버그를 자동 검출 못 함.

## 6. 후속 권고

- 작성자가 `update_ctrl_dimensions` 의 width/height 오프셋
  `[12..16]/[16..20]` 로 정정 + 주석 갱신
- `set_table_properties_native` outer_margin `[24..32]` 로 정정
- 회귀 가드를 width/height/outer_margin 까지 확장
- 표 크기 조절 → 저장 → 재오픈 시나리오 단위 테스트 추가
- 자매 PR #1078 도 함께 점검 부탁
