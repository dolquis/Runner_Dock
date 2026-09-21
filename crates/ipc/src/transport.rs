//! Windows named pipe の待受。
//!
//! 既定 ACL に依存せず、[`crate::security::owner_only_sddl`] で作った security
//! descriptor をインスタンスごとに付ける。remote client も毎回拒否する
//! （`docs/09_SECURITY.md` TH-03、`docs/10_IPC_DATA_MODEL.md` §1）。
//!
//! 2 件目以降のインスタンスにも同じ属性を付けるのが要点である。最初の 1 つだけ
//! に付けると、接続のたびに既定 ACL の pipe が生えて境界が外れる。

// pipe へ security attributes を渡す経路だけが unsafe になる。
#![expect(
    unsafe_code,
    reason = "SDDL の変換と security attributes 付きの pipe 生成は Win32 API 経由になる"
)]

use std::io;
use std::ptr;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;

use crate::name::pipe_name;
use crate::security::owner_only_sddl;

/// 同時に開ける pipe インスタンスの上限。
///
/// 無制限にしない。現在の待受は待機 1 本と受理済み 1 本しか持たないので、
/// この値が効くのは多重化を入れた後（LF-009）である。先に上限を置いておく。
pub const MAX_PIPE_INSTANCES: u32 = 16;

/// `LocalFree` が要る security descriptor。
struct SecurityDescriptor(*mut core::ffi::c_void);

impl SecurityDescriptor {
    /// SDDL 文字列から作る。
    fn from_sddl(sddl: &str) -> io::Result<Self> {
        let wide: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
        let mut descriptor: *mut core::ffi::c_void = ptr::null_mut();
        // SAFETY: `wide` は NUL 終端の UTF-16 で、変換中だけ読まれる。出力の
        // 置き場所は書き込み可能で、成功時の buffer は Drop で解放する。
        // 参照: Win32 `ConvertStringSecurityDescriptorToSecurityDescriptorW`。
        let converted = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                SDDL_REVISION_1,
                &raw mut descriptor,
                ptr::null_mut(),
            )
        };
        if converted == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(descriptor))
    }

    const fn as_ptr(&self) -> *mut core::ffi::c_void {
        self.0
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: 変換が成功したときだけ構築するので、`LocalAlloc` 由来の
        // 有効な値である。Drop は 1 度しか走らない。
        unsafe {
            LocalFree(self.0);
        }
    }
}

/// 所有者だけが接続できる named pipe の待受。
pub struct PipeListener {
    name: String,
    sddl: String,
    /// 次の接続を待っているインスタンス。
    ///
    /// 次のインスタンスを作れなかった場合に `None` になる。待受を失ったまま
    /// 黙って動き続けないよう、次の [`PipeListener::accept`] で作り直す。
    pending: Option<NamedPipeServer>,
}

impl PipeListener {
    /// SID から名前と DACL を決めて待受を開く。
    ///
    /// 最初のインスタンスを `first_pipe_instance` で作るので、同じ名前の pipe が
    /// 既にあれば失敗する。これが単一起動の判定そのものになる（pipe の名前空間
    /// はマシン全体で 1 つなので、別のログオンセッションからの二重起動も止まる）。
    ///
    /// # Errors
    ///
    /// SID が不正なとき、pipe を作れないときに返す。既に同名の pipe があるときは
    /// [`io::ErrorKind::AddrInUse`] 相当の OS エラーになる。
    pub fn bind(sid: &str) -> io::Result<Self> {
        let name = pipe_name(sid).map_err(io::Error::other)?;
        let sddl = owner_only_sddl(sid).map_err(io::Error::other)?;
        let pending = create_instance(&name, &sddl, true)?;
        Ok(Self {
            name,
            sddl,
            pending: Some(pending),
        })
    }

    /// 待受けている pipe 名。ログと診断に使う。秘密ではない。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 次の接続を受ける。
    ///
    /// 接続が成立したインスタンスを返し、自分は新しいインスタンスを用意する。
    /// 新しい方にも同じ security attributes と remote client 拒否を付ける。
    ///
    /// # Errors
    ///
    /// 接続待ちに失敗したとき、次のインスタンスを作れないときに返す。
    pub async fn accept(&mut self) -> io::Result<NamedPipeServer> {
        // 前回、次のインスタンスを作れずに終わっていたらここで作り直す。
        let pending = match self.pending.take() {
            Some(pending) => pending,
            None => create_instance(&self.name, &self.sddl, false)?,
        };
        // 失敗した場合も待機インスタンスを捨てない。
        if let Err(error) = pending.connect().await {
            self.pending = Some(pending);
            return Err(error);
        }

        // 次を用意できなくても、成立した接続は返す。次回の accept で作り直す。
        match create_instance(&self.name, &self.sddl, false) {
            Ok(next) => self.pending = Some(next),
            Err(error) => {
                tracing::warn!(kind = ?error.kind(), "次の待受インスタンスを作れなかった");
            }
        }
        Ok(pending)
    }
}

/// インスタンスを 1 つ作る。属性は毎回同じものを付ける。
fn create_instance(name: &str, sddl: &str, first: bool) -> io::Result<NamedPipeServer> {
    let descriptor = SecurityDescriptor::from_sddl(sddl)?;
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(u32::MAX),
        lpSecurityDescriptor: descriptor.as_ptr(),
        // 子プロセスへ handle を継がせない。
        bInheritHandle: 0,
    };

    let mut options = ServerOptions::new();
    options
        // ネットワーク経由の接続を受けない（`docs/09_SECURITY.md` TH-03）。
        .reject_remote_clients(true)
        .first_pipe_instance(first)
        .max_instances(MAX_PIPE_INSTANCES as usize);

    // SAFETY: `attributes` は呼び出しの間だけ生きていればよく、その中の
    // descriptor も同じ scope で生かしている。tokio は渡された
    // `SECURITY_ATTRIBUTES` を `CreateNamedPipeW` へ転送するだけで、保持しない。
    // 参照: tokio `ServerOptions::create_with_security_attributes_raw`。
    let server =
        unsafe { options.create_with_security_attributes_raw(name, (&raw mut attributes).cast()) }?;
    drop(descriptor);
    Ok(server)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::current_user_sid;
    use tokio::net::windows::named_pipe::ClientOptions;

    /// 既に待受がある場合だけ「取れない」を許す。
    ///
    /// それ以外の失敗（名前の不正など）を「他が待受中」と読み替えると、pipe を
    /// 一度も作れていない実装でも試験が通ってしまう。
    fn allow_only_already_listening(error: &io::Error) {
        let code = error.raw_os_error();
        assert!(
            // ERROR_ACCESS_DENIED / ERROR_ALREADY_EXISTS / ERROR_PIPE_BUSY
            matches!(code, Some(5 | 183 | 231)),
            "待受を作れない理由が「既に待受中」ではない: {error:?}"
        );
    }

    /// 待受の性質を 1 つの試験で順に確かめる。
    ///
    /// pipe 名は SID から決まるので、分けて書くと同じ名前を並列に奪い合う。
    #[tokio::test]
    async fn the_listener_is_exclusive_and_accepts_the_owner() {
        let sid = current_user_sid().unwrap();
        let mut listener = match PipeListener::bind(&sid) {
            Ok(listener) => listener,
            Err(error) => {
                // 開発中の Agent が同じ名前で待受けている場合。その状況自体が
                // 「二重に待受けられない」ことの確認なので終える。
                allow_only_already_listening(&error);
                return;
            }
        };

        // 同じ名前で 2 つ目の待受は作れない。これが単一起動の一方の柱である。
        assert!(
            PipeListener::bind(&sid).is_err(),
            "同じ名前で 2 つ目の待受を作れてしまった"
        );

        // 所有者からは繋がる。DACL が自分自身まで閉めていないことの確認。
        let name = listener.name().to_owned();
        let client = ClientOptions::new().open(&name);
        assert!(client.is_ok(), "所有者から接続できない: {client:?}");
        assert!(listener.accept().await.is_ok());

        // 受理したあとも待受は続く。次のインスタンスが用意されている。
        let second_client = ClientOptions::new().open(&name);
        assert!(
            second_client.is_ok(),
            "接続を 1 件受けた後に待受が消えた: {second_client:?}"
        );
        assert!(listener.accept().await.is_ok());
    }

    #[tokio::test]
    async fn a_value_that_is_not_a_sid_does_not_produce_a_listener() {
        assert!(PipeListener::bind("not-a-sid").is_err());
    }
}
