# Task #1129 최종 보고서 - rhwp-studio 격자 보기 및 격자 설정 활성화

- 이슈: [#1129](https://github.com/edwardkim/rhwp/issues/1129)
- 브랜치: `local/task_m100_1129`
- 일자: 2026-05-26

## 1. 작업 결과

`rhwp-studio`에서 한컴오피스처럼 격자 보기를 켜고 끌 수 있게 했고, 격자 설정 창에서 모양/위치/방식/간격/기준 위치/오프셋을 설정할 수 있게 했다.

HWP5/HWPX 파서도 격자 관련 스펙 값을 버리지 않도록 보강했다.

## 2. 주요 변경

- 보기 메뉴와 도구막대 `격자 보기` 명령 활성화
- 페이지별 DOM 격자 오버레이 추가
- 격자 설정 상태 모델 추가
- 한컴오피스식 격자 설정 대화상자 확장
- `쪽`/`종이` 기준 위치 전환 기본값 적용
- HWP5 `SECTION_DEFINE`의 줄/글자 격자 보존
- HWPX `hp:grid` 및 `snapToGrid` 보존

## 3. 기준 위치 정책

- `쪽`: 본문 쪽 영역 기준. 격자 표시 범위를 본문 영역으로 clip 한다.
- `종이`: 종이 전체 기준. 격자 표시 범위를 페이지 전체로 둔다.
- `종이` 기준 전환 시 종이 기준 보정값을 mm로 환산해 기본 오프셋으로 사용
- 사용자가 직접 오프셋을 바꾼 경우에는 기준 위치 변경 시 사용자 값을 유지

## 4. 검증

- `cargo test test_parse_section_with_section_def --lib -- --nocapture` 통과
- `cargo test test_parse_section_grid_preserves_line_and_char_grid --lib -- --nocapture` 통과
- `cargo test test_parse_hwpx_para_shape_snap_to_grid_bit --lib -- --nocapture` 통과
- `cargo test --lib` 통과: 1397 passed, 0 failed, 6 ignored
- `cargo fmt --all -- --check && git diff --check` 통과
- `npm run build` 통과
- 로컬 Playwright 기능 검증 통과
  - `쪽`: overlay `clip-path: inset(...)`, `background-position: 128.539px 223.039px` (`3mm, 3mm` 입력)
  - `종이`: overlay `clip-path: none`, `background-position: 162.52px 132.283px` (`43mm, 35mm` 입력)
  - 격자 오버레이 생성 및 도구막대 active 상태 확인

## 5. 참고

- 시각적 정합 판단은 별도 수동 확인 대상으로 남긴다.
- 이슈 close는 작업지시자 승인 전까지 수행하지 않는다.
- `jangster77` 계정은 assignee 지정 권한이 없으므로 assignee 변경은 시도하지 않았다.
