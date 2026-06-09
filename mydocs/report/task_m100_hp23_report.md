# Task #23 최종 결과보고서 — upstream(rhwp) 엔진 동기화: devel 재베이스

- **이슈**: [paldyn/HanPage#23](https://github.com/paldyn/HanPage/issues/23) (M100)
- **브랜치**: `local/task23`(문서) · `local/task23-rebase`(재구축 product)
- **일자**: 2026-06-09
- **상태**: Stage 1~4 완료·검증 통과 → **본 최종 보고 승인 후 `devel` 반영(force-push)**

## 1. 개요

"upstream에서 업데이트되는 내용을 반영할 수 있는가?"라는 작업지시에서 출발. 조사 결과 fork가 upstream 엔진보다 **33개 PR 뒤처졌고**, fork가 upstream 히스토리를 **리플레이**한 구조라 단순 `git merge`가 불가능(merge-base가 옛 분기점이라 1665개 patch-동일 커밋이 거짓 충돌로 등록)했다. 작업지시자 결정에 따라 **재베이스 전략**(upstream/devel을 새 기반으로 삼고 paldyn 레이어만 재적용)을 채택.

- **목표**: `devel`을 `upstream/devel` 최신 엔진 기준으로 재구축 + paldyn 레이어(리브랜딩·HanPage-Desktop·CI/Pages·mydocs) 재적용. 향후 동기화 영구 단순화.

## 2. 핵심 조사 결과

- **fork 구조**: upstream 히스토리 리플레이(1665 patch-동일) + 얇은 paldyn 레이어(main 고유 56커밋 = 리브랜딩 + HanPage-Desktop + CI + 엔진fix 10).
- **분기점**: `854515f5`(2026-04-08, Task #76). 고정 기준 upstream tip = `f6ffe9d6`.
- **main/devel 불일치**: main=완전 제품(`HanPage-Desktop/`), devel=stale(`rhwp-desktop/`). paldyn 레이어 권위 원본 = **main** → 재베이스로 불일치 해소.
- **엔진fix 10건 triage = 전부 DROP**: 모두 upstream/devel에 동등 이상 반영(리플레이 8·대체 1·인프라 1) → paldyn 레이어 = **순수 비-엔진**(엔진 0줄 이식). 이 판정은 Stage 4의 **엔진 src diff=0 + 1933 테스트 통과**로 역증명.

## 3. 메커닉 = rebuild-fresh

데스크톱 saga 중간 상태를 재현하지 않고 최종 상태를 얇은 신규 커밋으로 재구성:
1. `local/task23-rebase` ← 고정 upstream/devel(`f6ffe9d6`).
2. `HanPage-Desktop/` 최종 상태 이식(자기완결).
3. 리브랜딩 변환을 upstream 신버전 위에 재적용(net 결과).
4. CI/Pages 재적용.
5. paldyn mydocs 반영.

## 4. 단계별 결과

| 단계 | 내용 | product 커밋 | 결과 |
|------|------|-------------|------|
| **Stage 1** | 기준 고정 + 재베이스 브랜치 + triage | (기반 `f6ffe9d6`) | sanity 빌드 26.95s |
| **Stage 2** | HanPage-Desktop 이식(28파일, 자기완결) | `c8ebe7f2` | 엔진 충돌 0 |
| **Stage 3** | 리브랜딩 + CI/Pages + studio 데스크톱 글루(실측 46파일) | `50074b26`·`5e921ade` | 엔진 src diff=0·식별자 보존·Pages 격리·보안위생 |
| **Stage 4** | 검증 | — | build/test 1933 pass·diff=0·glue 타입체크·33PR 흡수·무손실 |
| **Stage 5** | 최종 보고 + mydocs 반영 → 승인 → devel force-push | (본 문서) | 승인 후 실행 |

### Stage 3 주요 판단
- 비-엔진 차이 ~123파일 중 **대부분이 upstream studio 진화**(33 PR 신규 다이얼로그·핸들러) → 유지. 실제 편집/채택 46파일을 **3-버킷**(A 유지·B 채택·C 외과)으로 분리.
- **브랜드 변환**(서비스 표면만): 레포→`paldyn/HanPage`, 도메인→`hanpage.paldyn.com`, 제품명→HanPage, H-마크. **보존**: crate `rhwp`·`@rhwp/*`·`edwardkim.rhwp-vscode`·Edward Kim 저작권·엔진 크레딧 링크.
- **Stage 2 스코프 보정**: studio측 데스크톱 글루(`desktop-bridge.ts`·`file.ts` +51·`main.ts` 배선)를 누락분으로 재적용.
- **보안 위생**: cert/key gitignore + 커밋된 자체서명 개인키 제거(시크릿-금지 강화).

## 5. 검증 결과 (Stage 4)

| 검증 | 결과 |
|------|------|
| 엔진 `src/` diff vs upstream `f6ffe9d6` | **0** (byte-identical) |
| `cargo build`(네이티브) | ✅ rhwp v0.7.13, 9.05s |
| `cargo test`(네이티브) | ✅ **1933 passed / 0 failed** |
| studio `tsc` 타입체크 | 변경 파일 0 에러(잔여 4건 = stale pkg drift, CI 재빌드 해소) |
| 누락 33 PR 흡수 | ✅ #1220/1221/1222/1228 + `cell_path_json` |
| 무손실 | ✅ HanPage-Desktop 28·CNAME·LICENSE·폰트라이선스·H-마크·Pages 격리 |
| 엔진 식별자 | ✅ crate rhwp·@rhwp/editor·edwardkim |
| 추적 시크릿/개인키 | ✅ 0 |

## 6. 보존 불변식 점검

| 불변식 | 상태 |
|--------|------|
| rhwp 엔진 식별자(crate/`@rhwp`/edwardkim/저작권) | 보존 ✅ |
| paldyn 브랜딩(HanPage·hanpage.paldyn.com·H-마크) | 적용 ✅ |
| HanPage-Desktop 재적용 | 28파일 + studio 글루 ✅ |
| GitHub Pages 무영향 | `deploy-pages.yml`은 `push:[main]` 트리거(devel push 무관) + `HanPage-Desktop/**` paths-ignore ✅ |
| 시크릿 금지 | 신규 0 + 기존 개인키 제거(강화) ✅ |
| 엔진 src 무변경 | diff=0 ✅ |

## 7. devel 반영 (승인 후 실행)

mydocs 반영(§2-4)을 포함해 `local/task23-rebase`를 완성한 뒤, **본 최종 보고 승인 후**에만 아래 실행:

```bash
# 1) 백업 태그 (롤백 보장)
git tag backup/devel-pre-task23 origin/devel
# 2) devel 갱신 (force-push — 되돌릴 수 없음)
git branch -f devel local/task23-rebase   # 또는 devel 체크아웃 후 reset --hard
git push --force-with-lease origin devel
# 3) 검증: 원격 devel = local/task23-rebase, 엔진 src=upstream, 빌드/테스트
```

- **mydocs 반영**: upstream mydocs(엔진 기록) 유지 + paldyn 전용 31파일 + 본 보고서 + paldyn orders 4건(0530·0531·0601·0602) 추가. 엔진 문서(`tech/hwp_save_guide.md` 등) 충돌은 upstream 우선(퇴행 방지).
- **롤백**: 문제 시 `git push --force-with-lease origin devel backup/devel-pre-task23`로 즉시 환원.

## 8. 비범위

- **main**: 본 작업 미변경. devel 검증·반영 후 별도 릴리스 PR로 처리.
- **WASM/데스크톱 풀빌드**: CI/릴리스 시점(`deploy-pages.yml`·`desktop-release.yml`). 본 작업은 네이티브 빌드/테스트 + studio 타입체크까지.

## 9. 효과 — 향후 동기화 단순화

재베이스 후 devel의 merge-base가 upstream 최신 tip이 되므로, 차기 upstream 동기화는 일반 `git merge upstream/devel`로 가능(리플레이 거짓 충돌 소멸). paldyn 레이어가 얇고 비-엔진으로 격리돼 충돌면이 브랜딩/문서로 한정된다.

## 10. 산출물

- 계획서: `plans/task_m100_hp23.md`·`plans/task_m100_hp23_impl.md`
- 단계 보고서: `working/task_m100_hp23_stage{1,2,3,4}.md`
- 최종 보고서: 본 문서
- product 브랜치: `local/task23-rebase`(devel 후보)
