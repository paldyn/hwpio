# 최종 보고서 - Task M100-1197

- 이슈: #1197
- 제목: HWPX 용지 기준 BehindText 그림/표 z-order 보존
- 브랜치: `local/task1197`
- 작성일: 2026-06-02
- 상태: 자동 검증 완료, 작업지시자 시각검증 대기

## 1. 문제

HWPX 문서의 용지/페이지 기준 anchored 객체가 Picture/Table/Shape 타입별 경로로 렌더되면서
같은 `textWrap`/`zOrder` 축에서 합성되지 않았다. 그 결과 낮은 z-order BehindText 표 텍스트가
전체 페이지 이미지 위에 다시 표시되고, InFrontOfText 도형도 의도한 순서로 보존되지 않을 수 있었다.

## 2. 해결

`RenderNode` 공통 레이어 메타데이터를 추가하고, layout/SVG/PaintOp 경로가 동일한 레이어 계약을 따르도록 연결했다.

주요 변경:

- `RenderLayerInfo { text_wrap, z_order, stable_index }` 추가
- 용지/페이지 기준 Picture/Table/Shape top-level node에 layer metadata stamp
- paper/page anchored render node 정렬 키를 `(plane, z_order, stable_index)`로 통일
- SVG renderer가 `RenderNode.layer`를 우선 사용해 plane/z-order 정렬
- `LayerNode.layer` 추가 및 `LayerBuilder` lowering 시 metadata 보존
- PaintOp replay plane 계산을 `paint_op_replay_plane_with_layer()`로 확장
- CanvasKit/native Skia/WebCanvas가 inherited layer 기준으로 replay plane을 판단
- PageLayerTree JSON에 optional `layer` metadata 직렬화

## 3. 검증

통과한 주요 명령:

```sh
cargo fmt --all --check
cargo test --test issue_1167_svg_behindtext_zorder -- --nocapture
cargo test --test issue_1197_svg_object_zorder -- --nocapture
cargo test --tests
cargo test --features native-skia --lib behind_text_layered_vector_replays_below_flow_across_tree_branches -- --nocapture
```

비고:

- #1167 테스트는 기존 `LAYOUT_OVERFLOW` 진단 1건을 출력하지만 assertion은 통과했다.
- 원본 `[2027] 온새미로 1 본교재.hwpx`와 PDF 정답지는 저장소 및 #1197 이슈 본문에 직접 첨부되어 있지 않아 원본 기반 export는 수행하지 못했다.

## 4. 시각검증 산출물

작업지시자 확인용 산출물:

- `output/poc/issue1197/visual_check.html`
- `output/poc/issue1197/synthetic/issue1197_synthetic_zorder.svg`
- `output/poc/issue1197/issue1167/복학원서.svg`

확인 기준:

- #1197 synthetic: 낮은 `Z01_LOW_TABLE`은 파란 `Z11 IMAGE` 아래에 가려지고, `Z12_FINAL_TABLE`과 `01`은 위에 보인다.
- #1167 실제 샘플: BehindText 워터마크가 본문 텍스트를 덮지 않는다.

## 5. 커밋

- `9b330e71` Task #1197: add z-order red test
- `7efb2528` Task #1197: add render layer metadata
- `1628058d` Task #1197: stamp paper object layers
- `58795584` Task #1197: replay layered paint order

Stage 5 문서 커밋은 본 보고서, Stage 5 완료보고서, orders 갱신을 포함한다.

## 6. 남은 결정

시각검증 승인 후 다음 중 하나를 진행한다.

- PR 생성
- 추가 원본 파일이 제공되면 원본 HWPX/PDF 기반 SVG 대조 검증
- 작업지시자 승인 시 issue close
