//! 実行中プロセスの SID を取る。
//!
//! pipe 名と DACL の両方がこの値から決まる（[`crate::name`]、[`crate::security`]）。
//! 環境変数やユーザー名から推定せず、token から引く。名前は改名で変わりうるが
//! SID は変わらないためで、`docs/09_SECURITY.md` TH-03 の SID 照合もこの値を指す。

// OS の token API を呼ぶためだけに unsafe を使う。ここ以外へ広げない
// （`AGENTS.md` §5.2）。
#![expect(
    unsafe_code,
    reason = "SID の取得は Win32 の token API 経由でしか行えない"
)]

use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// 閉じ忘れを防ぐための token handle。
struct TokenHandle(HANDLE);

impl Drop for TokenHandle {
    fn drop(&mut self) {
        // SAFETY: `OpenProcessToken` が成功したときだけ構築するので、値は
        // 有効な handle である。Drop は 1 度しか走らないため二重解放しない。
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// 実行中プロセスのユーザー SID を文字列で返す。
///
/// # Errors
///
/// token を開けない、token 情報を取れない、SID を文字列化できないときに
/// OS のエラーをそのまま返す。
pub fn current_user_sid() -> io::Result<String> {
    let mut raw_token: HANDLE = ptr::null_mut();
    // SAFETY: `GetCurrentProcess` は疑似 handle を返すだけで解放が要らない。
    // 第 3 引数には書き込み可能な `HANDLE` の場所を渡している。
    // 参照: Win32 `OpenProcessToken`。
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut raw_token) };
    if opened == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = TokenHandle(raw_token);

    let mut needed: u32 = 0;
    // SAFETY: buffer に null、長さに 0 を渡す問い合わせは、必要な大きさを
    // `needed` へ書いて失敗する既定の使い方である。戻り値は次の分岐で見る。
    // 参照: Win32 `GetTokenInformation`。
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &raw mut needed);
    }
    if needed == 0 {
        return Err(io::Error::last_os_error());
    }

    // `TOKEN_USER` として読むので、`u8` の Vec ではなく整列の保証がある器を使う。
    let words = (needed as usize).div_ceil(size_of::<u64>()).max(1);
    let mut buffer: Vec<u64> = vec![0; words];
    // SAFETY: buffer は `needed` バイト以上を占め、`u64` 整列なので
    // `TOKEN_USER` の整列要件を満たす。長さも同じ値を渡している。
    let filled = unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &raw mut needed,
        )
    };
    if filled == 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: 直前の呼び出しが成功しているので、buffer の先頭には有効な
    // `TOKEN_USER` があり、その `User.Sid` は同じ buffer 内を指す。参照は
    // `buffer` が生きている間だけ使う。
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };

    let mut text: *mut u16 = ptr::null_mut();
    // SAFETY: `sid` は上で得た有効な SID を指し、`text` は書き込み可能な
    // ポインタの場所である。成功時の buffer は `LocalFree` で返す。
    // 参照: Win32 `ConvertSidToStringSidW`。
    let converted = unsafe { ConvertSidToStringSidW(sid, &raw mut text) };
    if converted == 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: 成功した `ConvertSidToStringSidW` は NUL 終端の UTF-16 文字列を
    // 返す。長さを数えてから読み、読み終えてから解放する。
    let value = unsafe {
        let mut length = 0_usize;
        while *text.add(length) != 0 {
            length += 1;
        }
        let slice = std::slice::from_raw_parts(text, length);
        let value = String::from_utf16_lossy(slice);
        LocalFree(text.cast());
        value
    };
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::validate_sid;

    #[test]
    fn returns_a_well_formed_sid_for_the_current_process() {
        let sid = current_user_sid().unwrap();

        assert!(validate_sid(&sid).is_ok(), "{sid}");
        assert!(sid.starts_with("S-1-"), "{sid}");
    }

    #[test]
    fn the_sid_is_stable_within_a_process() {
        assert_eq!(current_user_sid().unwrap(), current_user_sid().unwrap());
    }
}
