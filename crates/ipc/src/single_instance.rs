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
    /// 同じ SID の Agent が既に動いている。
    AlreadyRunning,
    /// mutex を作れなかった。
    Os(io::Error),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => f.write_str("同じ利用者の Agent が既に動いている"),
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
        let name =
            single_instance_name(sid).map_err(|error| LockError::Os(io::Error::other(error)))?;
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

        // SAFETY: security attributes を渡さないので既定（作成者のみ）になる。
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

    #[test]
    fn a_second_lock_for_the_same_sid_is_refused() {
        let sid = current_user_sid().unwrap();
        let first = match SingleInstanceLock::acquire(&sid) {
            Ok(lock) => lock,
            // 開発中の Agent が動いている場合。取れないこと自体が期待動作。
            Err(LockError::AlreadyRunning) => return,
            // 名前が不正で作れない等を「既に動いている」と読み替えない。
            Err(LockError::Os(error)) => panic!("mutex を作れない: {error:?}"),
        };

        let second = SingleInstanceLock::acquire(&sid);

        assert!(
            matches!(second, Err(LockError::AlreadyRunning)),
            "{second:?}"
        );
        drop(first);
    }

    #[test]
    fn the_lock_is_released_when_dropped() {
        let sid = current_user_sid().unwrap();
        let first = match SingleInstanceLock::acquire(&sid) {
            Ok(lock) => lock,
            Err(LockError::AlreadyRunning) => return,
            Err(LockError::Os(error)) => panic!("mutex を作れない: {error:?}"),
        };
        drop(first);

        assert!(SingleInstanceLock::acquire(&sid).is_ok());
    }
}
