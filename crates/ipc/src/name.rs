//! pipe と単一起動ロックの名前。
//!
//! `docs/10_IPC_DATA_MODEL.md` §1 の例に合わせ `\\.\pipe\runnerdock.<sid-hash>.v1`
//! を使う。SID をそのまま名前へ入れないのは、pipe 名が列挙可能で、利用者の
//! アカウント識別子を不要に広げないためである。名前を秘密として扱わないことは
//! 変わらない（`docs/09_SECURITY.md` TH-03）。

use crate::security::{SecurityError, validate_sid};

/// pipe 名に載せる protocol の版。`PROTOCOL_MAJOR` と同じ値を使う。
///
/// major が上がった Agent は別の pipe で待受ける。旧 UI が新 Agent へ繋いで
/// 復号段階で落ちるより、そもそも別の待受にしておく方が診断しやすい。
pub const PIPE_VERSION_TAG: &str = "v1";

/// SID から pipe 名を決める。
///
/// # Errors
///
/// SID の書式が `S-1-...` でないときに [`SecurityError`] を返す。
pub fn pipe_name(sid: &str) -> Result<String, SecurityError> {
    validate_sid(sid)?;
    Ok(format!(
        r"\\.\pipe\runnerdock.{}.{PIPE_VERSION_TAG}",
        sid_hash(sid)
    ))
}

/// SID から単一起動ロックの名前を決める。
///
/// `Local\` を使う。`Global\` の名前空間へ作るには `SeCreateGlobalPrivilege` が
/// 要り、非昇格で動く Agent は持たない（`AGENTS.md` §5.1、ADR-003）。
/// この mutex は同一ログオンセッション内の即時判定で、ログオンセッションを
/// 跨ぐ排他は pipe の最初のインスタンス生成が担う。pipe の名前空間はマシン全体で
/// 1 つなので、2 人目の待受は生成時点で失敗する。
pub fn single_instance_name(sid: &str) -> Result<String, SecurityError> {
    validate_sid(sid)?;
    Ok(format!(
        r"Local\runnerdock.{}.{PIPE_VERSION_TAG}",
        sid_hash(sid)
    ))
}

/// SID の短縮表現。FNV-1a 64bit を 16 桁の 16 進で表す。
///
/// 暗号学的ハッシュではない。ここで守りたいのは「名前に SID をそのまま出さない」
/// ことだけで、衝突しても別 SID の接続が通るわけではない（接続可否は DACL が
/// 決める）。依存を増やさずに、同じ SID から常に同じ名前が出ることを優先する。
#[must_use]
pub fn sid_hash(sid: &str) -> String {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    // SID の文字列表現は大文字小文字を区別しないので、比較の前に畳む。
    let mut hash = OFFSET_BASIS;
    for byte in sid.to_ascii_uppercase().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID: &str = "S-1-5-21-1111111111-2222222222-3333333333-1001";

    #[test]
    fn pipe_name_follows_the_documented_shape() {
        let name = pipe_name(SID).unwrap();

        // 先頭は `\\.\pipe\`（バックスラッシュ 2 つ、ドット、バックスラッシュ）で
        // なければならない。1 つ足りないと `CreateNamedPipeW` が
        // `ERROR_INVALID_NAME` で失敗する。文字単位で確かめる。
        assert_eq!(
            name.chars().take(9).collect::<String>(),
            ['\\', '\\', '.', '\\', 'p', 'i', 'p', 'e', '\\']
                .iter()
                .collect::<String>(),
            "{name}"
        );
        assert!(name.starts_with(r"\\.\pipe\runnerdock."), "{name}");
        assert!(name.ends_with(".v1"), "{name}");
    }

    #[test]
    fn pipe_name_does_not_leak_the_raw_sid() {
        assert!(!pipe_name(SID).unwrap().contains(SID));
    }

    #[test]
    fn the_same_sid_always_maps_to_the_same_name() {
        assert_eq!(pipe_name(SID).unwrap(), pipe_name(SID).unwrap());
        assert_eq!(
            pipe_name(SID).unwrap(),
            pipe_name(&SID.to_ascii_lowercase()).unwrap()
        );
    }

    #[test]
    fn different_sids_map_to_different_names() {
        let other = "S-1-5-21-1111111111-2222222222-3333333333-1002";

        assert_ne!(pipe_name(SID).unwrap(), pipe_name(other).unwrap());
    }

    #[test]
    fn rejects_a_value_that_is_not_a_sid() {
        assert!(pipe_name("not-a-sid").is_err());
        assert!(single_instance_name("").is_err());
    }

    #[test]
    fn single_instance_lock_lives_in_the_local_namespace() {
        let name = single_instance_name(SID).unwrap();

        assert!(name.starts_with(r"Local\runnerdock."), "{name}");
        assert!(!name.contains(SID));
    }
}
