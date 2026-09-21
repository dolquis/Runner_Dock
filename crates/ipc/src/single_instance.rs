//! 単一起動の強制。
//!
//! 同一 SID あたり Agent を 1 インスタンスに限る（`docs/03_ARCHITECTURE.md` §3）。
//! 古い PID ファイルの有無で判断しない。OS の named mutex と、named pipe の
//! 最初のインスタンス生成（[`crate::transport::PipeListener::bind`]）を併用する。
//!
//! 役割はこう分かれている。mutex は同じログオンセッション内の即時判定で、
//! プロセスが落ちれば OS が handle を回収するので取り残しが起きない。pipe の
//! 方はマシン全体の名前空間なので、ログオンセッションを跨いだ二重起動も止まる。

// named mutex の取得だけが unsafe になる。
#![expect(
    unsafe_code,
    reason = "named mutex の取得と解放は Win32 API 経由になる"
)]

use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;

use crate::name::single_instance_name;

/// 取得済みの単一起動ロック。落とすと解放する。
#[derive(Debug)]
pub struct SingleInstanceLock {
    handle: HANDLE,
    name: String,
}

/// ロックを取れなかった理由。
#[derive(Debug)]
pub enum LockError {
    /// 同じ名前の単一起動ロックが既にある。
    ///
    /// 通常は同じ利用者の Agent が動いている場合だが、名前は SID から決まり
    /// 予測できるので、同一セッションの別プロセスが先に取った可能性も残る。
    /// 「Agent が動いている」と断定しない。
    AlreadyRunning,
    /// SID として解釈できず、ロック名を決められない。
    MalformedSid,
    /// mutex を作れなかった。
    Os(io::Error),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => f.write_str("同名の単一起動ロックが既に存在する"),
            Self::MalformedSid => f.write_str("SID からロック名を決められない"),
            Self::Os(error) => write!(f, "単一起動ロックを取得できない: {error}"),
        }
    }
}

impl std::error::Error for LockError {}

impl SingleInstanceLock {
    /// SID から決まる名前でロックを取る。
    ///
    /// # Errors
    ///
    /// 既に同じ名前の mutex があるときは [`LockError::AlreadyRunning`]、OS 側で
    /// 失敗したときは [`LockError::Os`] を返す。
    pub fn acquire(sid: &str) -> Result<Self, LockError> {
        let name = single_instance_name(sid).map_err(|_| LockError::MalformedSid)?;
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

        // SAFETY: security attributes を渡さないので、作成者 token の既定 DACL
        // が付く（作成者のほか SYSTEM と Administrators を含む）。ここで守るのは
        // 秘密ではなく「同名を 2 つ作らせない」ことだけなので、この既定でよい。
        // 名前は NUL 終端の UTF-16 で、呼び出しの間だけ読まれる。
        // 参照: Win32 `CreateMutexW`。
        let handle = unsafe { CreateMutexW(ptr::null(), 1, wide.as_ptr()) };
        if handle.is_null() {
            return Err(LockError::Os(io::Error::last_os_error()));
        }
        // SAFETY: 直前の呼び出しの結果を読むだけ。
        let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if already {
            // 既存の Agent の mutex を握ったままにしない。
            // SAFETY: `CreateMutexW` が返した有効な handle を 1 度だけ閉じる。
            unsafe {
                CloseHandle(handle);
            }
            return Err(LockError::AlreadyRunning);
        }
        Ok(Self { handle, name })
    }

    /// 取得したロックの名前。診断に使う。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for SingleInstanceLock {
    fn drop(&mut self) {
        // SAFETY: 取得に成功したときだけ構築するので有効な handle である。
        // Drop は 1 度しか走らないため二重解放しない。
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::current_user_sid;

    /// 取得と解放を 1 つの試験で順に確かめる。
    ///
    /// ロック名は SID から決まるので、分けて書くと同じ名前を並列に奪い合い、
    /// 「解放されたはずなのに取れない」偽の失敗が出る。
    #[test]
    fn the_lock_excludes_a_second_holder_and_is_released_on_drop() {
        let sid = current_user_sid().unwrap();
        let first = match SingleInstanceLock::acquire(&sid) {
            Ok(lock) => lock,
            // 開発中の Agent が動いている場合。取れないこと自体が期待動作。
            Err(LockError::AlreadyRunning) => return,
            // 名前が不正で作れない等を「既に動いている」と読み替えない。
            Err(error) => panic!("mutex を作れない: {error:?}"),
        };

        let second = SingleInstanceLock::acquire(&sid);
        assert!(
            matches!(second, Err(LockError::AlreadyRunning)),
            "{second:?}"
        );

        drop(second);
        drop(first);
        assert!(SingleInstanceLock::acquire(&sid).is_ok());
    }

    #[test]
    fn a_value_that_is_not_a_sid_is_not_reported_as_already_running() {
        let error = SingleInstanceLock::acquire("not-a-sid");

        assert!(matches!(error, Err(LockError::MalformedSid)), "{error:?}");
    }
}
