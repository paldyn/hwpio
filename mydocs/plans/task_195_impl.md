# Task #195: 차트/OLE 개체 렌더링 — 구현계획서

> 수행계획서: [task_195.md](task_195.md)
> 마일스톤: 미지정

## 전체 개요

5단계로 진행한다. 각 단계는 독립적으로 커밋 가능하며, 단계 완료 후 단계별 완료보고서(`_stageN.md`)를 작성해 승인받은 뒤 다음 단계로 진행한다.

| 단계 | 제목 | 산출물 | 커밋 단위 |
|------|------|--------|----------|
| 1 | 스펙 조사 및 IR 설계 | tech 문서 2건 | 문서만 |
| 2 | Model 계층: ChartShape / OleShape 구조체 | model/shape.rs 확장 | model 추가 |
| 3 | Parser 계층: CHART_DATA / OLE 파싱 | parser 신규 파일 2건 + shape.rs 분기 | parser 완성 |
| 4 | Renderer 계층: SVG 출력 | renderer 분기 + 단위 테스트 | 1차 렌더 |
| 5 | 검증 및 자체 제작 샘플 추가 | samples/chart-*.hwp + 단계별 보고서 + 최종 보고서 | 마무리 |

## 단계 1: 스펙 조사 및 IR 설계

### 목적
CHART_DATA / SHAPE_COMPONENT_OLE 바이너리 스펙을 조사하고, rhwp 내부 표현(IR)을 확정한다. **코드 변경 없음**, 문서만 작성.

### 작업 항목
1. `mydocs/tech/hwp_chart_spec.md` 작성
   - HWP 5.0 파일 포맷 스펙 문서 기반 CHART_DATA 레코드 구조 정리
   - 차트 종류(막대/선/파이/영역/분산형 등) enum
   - 데이터 시리즈, 축, 레이블, 범례 필드
   - 참조: pyhwp / hwplib / HWP 스펙 PDF
2. `mydocs/tech/hwp_ole_spec.md` 작성
   - SHAPE_COMPONENT_OLE 레코드 구조
   - BinData 스트림의 OLE 컨테이너(CFB) 구조
   - 프리뷰 이미지 추출 경로(Compound File \001CompObj / \005SummaryInformation / Ole10Native 등)
3. IR 설계 표
   - `ChartShape { common, drawing, chart_type, series: Vec<DataSeries>, axes, title, legend, ... }`
   - `OleShape { common, drawing, bin_item_id, preview: Option<Vec<u8>>, preview_format: ImageFormat }`

### 검증
- 문서 2건이 HWP 5.0 공식 스펙과 pyhwp 구현과 일치하는지 자체 리뷰
- IR 설계가 기존 `ShapeObject` 패턴(Line/Rectangle/Picture)과 일관성 있는지

### 산출물 커밋
- `mydocs/tech/hwp_chart_spec.md`
- `mydocs/tech/hwp_ole_spec.md`
- `mydocs/working/task_195_stage1.md` (단계별 완료보고서)

## 단계 2: Model 계층 확장

### 목적
`ShapeObject` enum에 `Chart`, `Ole` variant를 추가하고 기본 필드와 impl 블록을 구성한다.

### 작업 항목
1. `src/model/shape/chart.rs` 신규 — `ChartShape` 구조체, `ChartType` enum, `DataSeries` 등
2. `src/model/shape/ole.rs` 신규 — `OleShape` 구조체, `OleFormat` enum
3. `src/model/shape.rs` 수정
   - `ShapeObject` enum에 `Chart(Box<ChartShape>)`, `Ole(Box<OleShape>)` 추가
   - `common()`, `common_mut()`, `drawing()`, `drawing_mut()`, `shape_attr()` 등 기존 매치 arm 확장
4. `src/model/control.rs`의 Control enum은 `Shape(Box<ShapeObject>)`로 이미 감싸므로 변경 없음 확인

### 검증
- `cargo build --release` 컴파일 성공
- `cargo test` 기존 테스트 회귀 없음 (모델 추가만으로는 깨지지 않아야 함)

### 산출물 커밋
- 위 3개 파일
- `mydocs/working/task_195_stage2.md`

## 단계 3: Parser 계층 구현

### 목적
shape.rs의 `shape_tag_id` 분기에 CHART / OLE 처리를 추가한다.

### 작업 항목
1. `src/parser/control/shape_chart.rs` 신규
   - `parse_chart_shape(common, drawing, shape_tag_data, child_records) -> ChartShape`
   - child_records에서 `HWPTAG_CHART_DATA` 탐색 후 파싱
2. `src/parser/control/shape_ole.rs` 신규
   - `parse_ole_shape(common, drawing, shape_tag_data) -> OleShape`
   - BinData 참조 ID 추출, 프리뷰 이미지는 BinData 스트림에서 별도 로드
3. `src/parser/control/shape.rs` 수정
   - `Some(tags::HWPTAG_SHAPE_COMPONENT_OLE)` 분기 추가 → `parse_ole_shape` 호출
   - child_records에 `HWPTAG_CHART_DATA`가 있으면 차트로 인식 (차트는 GSO + CHART_DATA 조합)
   - 미지 분기(`_ =>`)는 유지하되 로그/diag 출력 추가
4. 단위 테스트
   - shape_chart.rs / shape_ole.rs 각각 고정 바이트 픽스처 테스트 (픽스처는 자체 제작 파일에서 추출)

### 검증
- `cargo build --release` 성공
- `cargo test` 신규 파서 단위 테스트 통과
- 로컬 1.hwp에 대해 `rhwp dump`로 "도형" 대신 "차트" / "OLE"로 식별되는지 확인
- 기존 샘플 회귀 없음

### 산출물 커밋
- 위 3개 파일
- `mydocs/working/task_195_stage3.md`

## 단계 4: Renderer 계층 SVG 출력

### 목적
파싱된 ChartShape / OleShape를 SVG로 렌더링한다.

### 작업 항목
1. `src/renderer/layout/shape_layout.rs`에 Chart / Ole 분기 추가
2. `src/renderer/svg_chart.rs` 신규 (또는 `svg.rs` 확장)
   - 1차 범위: 막대(세로/가로) / 선 / 파이
   - 축, 레이블, 범례, 타이틀 렌더
   - 제외 범위: 3D, 복합, 보조축
3. Ole 렌더링
   - 프리뷰 이미지가 있으면 `<image>` 태그로 placeholder 출력
   - 없으면 회색 사각형 + "OLE" 텍스트
4. `cargo test` 업데이트 — 스냅샷 테스트(가능 시)

### 검증
- 로컬 1.hwp `rhwp export-svg`로 차트 영역이 막대 그래프로 출력되는지 브라우저 확인
- 축/레이블 위치 HWP 뷰어와 육안 비교
- 기존 export-svg 회귀 없음

### 산출물 커밋
- 위 2개 파일 + 기존 수정
- `mydocs/working/task_195_stage4.md`

## 단계 5: 검증 및 마무리

### 목적
자체 제작 샘플로 회귀 테스트를 고정하고 최종 보고서를 작성한다.

### 작업 항목
1. 한컴오피스로 차트(막대/선/파이) 포함 HWP 자체 제작 → `samples/chart-basic.hwp`
2. E2E 회귀 스크립트에 chart-basic.hwp 추가
3. serializer 라운드트립 확인 (읽기 → 저장 → 읽기 동일성)
4. 전체 samples/ export-svg 회귀
5. `mydocs/working/task_195_stage5.md` + `task_195_report.md` (최종 보고서)
6. `mydocs/orders/` 오늘할일 갱신
7. GitHub Issue #195 close 준비 (승인 후 close)

### 검증
- samples/chart-basic.hwp → export-svg 정상
- 전체 samples/ 회귀 통과
- 단위 테스트 전체 green

### 산출물 커밋
- `samples/chart-basic.hwp`
- 보고서 문서
- 오늘할일 갱신

## 공통 규칙

- 각 단계 커밋 메시지: `Task #195: <단계 제목>`
- 단계별 완료 후 승인 없이 다음 단계 진행 금지
- 단계별 완료보고서에는 **실제 수정 파일 목록**, **테스트 결과**, **미해결 이슈** 포함
- 단계 5 완료 후 `local/task195` → `local/devel` merge는 작업지시자 승인 후 수행

## 리스크 및 대응

| 리스크 | 대응 |
|--------|------|
| CHART_DATA 스펙 일부 필드 미문서화 | pyhwp 구현 참조, 미지 필드는 raw 보존하여 라운드트립 유지 |
| OLE 프리뷰 이미지 포맷이 WMF/EMF | rhwp의 기존 WMF 파서(`src/wmf/`) 재사용 |
| 차트 종류가 많아 1차 범위 초과 | 막대/선/파이만 우선 구현, 나머지는 별도 이슈로 분리 |
| 자체 제작 샘플의 차트 종류 제한 | chart-basic.hwp에 최소 3종류 포함 |

## 승인 요청 항목

1. 5단계 분할이 적절한지
2. 각 단계 범위와 커밋 경계가 맞는지
3. 단계 1을 "문서만" 커밋하는 방식이 하이퍼-워터폴 규칙에 맞는지
4. 승인 시 단계 1(스펙 조사) 착수
