//! `CommonObjAttr` → CTRL_HEADER `raw_ctrl_data` 바이트 직렬화기.
//!
//! HWP 직렬화기 (`serializer/control.rs`) 는 `table.raw_ctrl_data` 를 그대로 기록한다.
//! HWPX 출처 표는 이 필드가 비어있으므로, 어댑터가 `CommonObjAttr` 으로부터 합성해야 한다.
//!
//! Stage 1: 골격만. Stage 2 에서 본격 구현.
//!
//! 본 모듈은 `parser/control/common_obj_attr.rs` (또는 등가 read 코드) 의 역방향이며,
//! 라운드트립 테스트로 검증한다.

use crate::model::shape::CommonObjAttr;

/// `CommonObjAttr` 을 CTRL_HEADER ctrl_data 영역 바이트로 직렬화.
///
/// Stage 1: TODO — 빈 Vec 반환. Stage 2 에서 실제 직렬화 작성.
pub fn serialize_common_obj_attr(_common: &CommonObjAttr) -> Vec<u8> {
    // Stage 2 에서 구현. 현재는 placeholder.
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage1_returns_empty_placeholder() {
        let common = CommonObjAttr::default();
        assert!(serialize_common_obj_attr(&common).is_empty());
    }
}
