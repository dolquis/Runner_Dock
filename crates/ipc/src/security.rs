//! pipe へ付ける DACL の決め方。
//!
//! named pipe の既定 ACL に依存せず、Agent の SID だけへ権限を与える
//! （`docs/09_SECURITY.md`「権限設計」、TH-03）。ここは SDDL 文字列を組み立てる
//! までを持ち、OS への適用は `transport` が行う。文字列の組み立てを分けておくと
//! Windows 以外でも意図を試験できる。

use thiserror::Error;

/// SID や SDDL の扱いで検出した不正。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SecurityError {
    /// SID 文字列として解釈できない。
    ///
    /// 相手由来の文字列をそのまま SDDL へ差し込むと、ACE を追加する形の
    /// 注入になりうる。組み立て前に必ず弾く。
    #[error("SID 文字列として解釈できない")]
    MalformedSid,
}

/// SID 文字列（`S-1-5-21-...`）として妥当か検査する。
///
/// 受け付けるのは `S-1-<権威>-<部分認証子>...` の 10 進表記だけである。SDDL の
/// 2 文字略称（`BA`、`WD` など）は、意図せず広い主体を指しうるので通さない。
///
/// # Errors
///
/// 書式が合わないときに [`SecurityError::MalformedSid`] を返す。
pub fn validate_sid(sid: &str) -> Result<(), SecurityError> {
    let mut parts = sid.split('-');
    if !parts
        .next()
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("S"))
    {
        return Err(SecurityError::MalformedSid);
    }
    if parts.next() != Some("1") {
        return Err(SecurityError::MalformedSid);
    }

    // 識別子権威。ここまでは主体を指さない。
    let authority = parts.next().ok_or(SecurityError::MalformedSid)?;
    if authority.is_empty() || !authority.bytes().all(|b| b.is_ascii_digit()) {
        return Err(SecurityError::MalformedSid);
    }

    let mut subauthorities = 0_usize;
    for part in parts {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(SecurityError::MalformedSid);
        }
        subauthorities += 1;
    }
    // 権威に続く部分認証子が最低 1 つないと主体を指さない。
    if subauthorities == 0 {
        return Err(SecurityError::MalformedSid);
    }
    Ok(())
}

/// 所有者だけへ全権を与える SDDL を組み立てる。
///
/// `D:P` で継承を切り、ACE は与えた SID の 1 件だけにする。Administrators や
/// SYSTEM を足さないのは、日常運用の IPC へ管理主体の経路を作らないためである
/// （`docs/09_SECURITY.md`「権限設計」）。
///
/// # Errors
///
/// SID の書式が不正なときに [`SecurityError::MalformedSid`] を返す。
pub fn owner_only_sddl(sid: &str) -> Result<String, SecurityError> {
    validate_sid(sid)?;
    Ok(format!("O:{sid}G:{sid}D:P(A;;GA;;;{sid})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID: &str = "S-1-5-21-1111111111-2222222222-3333333333-1001";

    #[test]
    fn grants_all_access_to_the_owner_only() {
        let sddl = owner_only_sddl(SID).unwrap();

        assert_eq!(sddl, format!("O:{SID}G:{SID}D:P(A;;GA;;;{SID})"));
        // ACE は 1 件だけ。別主体の ACE を足していない。
        assert_eq!(sddl.matches("(A;;").count(), 1);
    }

    #[test]
    fn the_dacl_is_protected_from_inheritance() {
        assert!(owner_only_sddl(SID).unwrap().contains("D:P("));
    }

    #[test]
    fn does_not_grant_well_known_administrative_principals() {
        let sddl = owner_only_sddl(SID).unwrap();

        for principal in [";BA)", ";SY)", ";WD)", ";AU)"] {
            assert!(!sddl.contains(principal), "{sddl} に {principal} がある");
        }
    }

    #[test]
    fn rejects_sddl_abbreviations_and_injection_attempts() {
        for candidate in [
            "BA",
            "WD",
            "",
            "S-1",
            // 権威だけで部分認証子が無い。主体を指さない。
            "S-1-5",
            "S-1-5-21-1001)(A;;GA;;;WD",
            "S-1-5-21-x",
            "S-2-5-21-1001",
        ] {
            assert_eq!(
                owner_only_sddl(candidate),
                Err(SecurityError::MalformedSid),
                "{candidate} を受理した"
            );
        }
    }

    #[test]
    fn accepts_a_well_formed_sid() {
        assert!(validate_sid(SID).is_ok());
        assert!(validate_sid("S-1-5-18").is_ok());
    }
}
