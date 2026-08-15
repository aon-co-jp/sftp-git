//! 本番サーバーへの実SFTP接続。`drift.rs`のマニフェスト突き合わせロジックへ
//! 実際にサーバー上のファイルを読み出してハッシュ化したマニフェストを渡す。
//!
//! 実サーバーへの接続が要るテストは`#[ignore]`とし、環境変数
//! (`SFTP_GIT_TEST_HOST`等)を設定した場合のみ手動実行する。

use crate::drift::Manifest;
use sha2::{Digest, Sha256};
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;

#[derive(Debug)]
pub enum SftpError {
    Connect(std::io::Error),
    Ssh(ssh2::Error),
    Io(std::io::Error),
}

impl std::fmt::Display for SftpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SftpError::Connect(e) => write!(f, "SFTPサーバーへの接続に失敗: {e}"),
            SftpError::Ssh(e) => write!(f, "SSH処理に失敗: {e}"),
            SftpError::Io(e) => write!(f, "ファイル読み出しに失敗: {e}"),
        }
    }
}

impl std::error::Error for SftpError {}

pub struct SftpConnection {
    session: Session,
}

impl SftpConnection {
    /// 公開鍵認証で本番サーバーへ接続する(推奨、パスワードを扱わない)。
    pub fn connect_with_private_key(
        host: &str,
        port: u16,
        username: &str,
        private_key_path: &std::path::Path,
        public_key_path: Option<&std::path::Path>,
    ) -> Result<Self, SftpError> {
        let tcp = TcpStream::connect((host, port)).map_err(SftpError::Connect)?;
        let mut session = Session::new().map_err(SftpError::Ssh)?;
        session.set_tcp_stream(tcp);
        session.handshake().map_err(SftpError::Ssh)?;
        session
            .userauth_pubkey_file(username, public_key_path, private_key_path, None)
            .map_err(SftpError::Ssh)?;
        Ok(Self { session })
    }

    /// パスワード認証で本番サーバーへ接続する(鍵認証が使えない場合のみ)。
    pub fn connect_with_password(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<Self, SftpError> {
        let tcp = TcpStream::connect((host, port)).map_err(SftpError::Connect)?;
        let mut session = Session::new().map_err(SftpError::Ssh)?;
        session.set_tcp_stream(tcp);
        session.handshake().map_err(SftpError::Ssh)?;
        session
            .userauth_password(username, password)
            .map_err(SftpError::Ssh)?;
        Ok(Self { session })
    }

    /// 指定したリモートディレクトリ配下のファイルを再帰的に読み出し、
    /// パス→SHA-256ハッシュのマニフェストを組み立てる。
    pub fn build_remote_manifest(&self, remote_dir: &str) -> Result<Manifest, SftpError> {
        let sftp = self.session.sftp().map_err(SftpError::Ssh)?;
        let mut manifest = Manifest::new();
        collect_manifest(&sftp, remote_dir, remote_dir, &mut manifest)?;
        Ok(manifest)
    }
}

fn collect_manifest(
    sftp: &ssh2::Sftp,
    base_dir: &str,
    current_dir: &str,
    manifest: &mut Manifest,
) -> Result<(), SftpError> {
    let entries = sftp
        .readdir(std::path::Path::new(current_dir))
        .map_err(SftpError::Ssh)?;

    for (path, stat) in entries {
        let path_str = path.to_string_lossy().to_string();
        if stat.is_dir() {
            collect_manifest(sftp, base_dir, &path_str, manifest)?;
            continue;
        }

        let mut remote_file = sftp.open(&path).map_err(SftpError::Ssh)?;
        let mut buf = Vec::new();
        remote_file.read_to_end(&mut buf).map_err(SftpError::Io)?;

        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let digest: [u8; 32] = hasher.finalize().into();
        let hash = digest.iter().map(|b| format!("{b:02x}")).collect::<String>();

        // Windows sshd(実機検証で確認)はSFTPパスをバックスラッシュ区切りで
        // 返すことがある。本番のLinuxサーバー(スラッシュ区切り)と
        // マニフェストのキー形式を一貫させるため、区切り文字を常に`/`へ
        // 正規化する。
        let relative = path_str
            .strip_prefix(base_dir)
            .unwrap_or(&path_str)
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();
        manifest.insert(relative, hash);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 実サーバーへの接続が必要なため既定では実行しない。
    /// `SFTP_GIT_TEST_HOST=127.0.0.1 SFTP_GIT_TEST_USER=... \
    ///  SFTP_GIT_TEST_KEY=/path/to/private_key SFTP_GIT_TEST_DIR=... \
    ///  cargo test -- --ignored` で手動実行する(公開鍵認証、パスワード不要)。
    #[test]
    #[ignore]
    fn build_manifest_against_real_server() {
        let host = std::env::var("SFTP_GIT_TEST_HOST").expect("SFTP_GIT_TEST_HOSTを設定してください");
        let user = std::env::var("SFTP_GIT_TEST_USER").expect("SFTP_GIT_TEST_USERを設定してください");
        let key_path = std::env::var("SFTP_GIT_TEST_KEY").expect("SFTP_GIT_TEST_KEYを設定してください");
        let remote_dir = std::env::var("SFTP_GIT_TEST_DIR").expect("SFTP_GIT_TEST_DIRを設定してください");

        let pub_key_path = format!("{key_path}.pub");
        let conn = SftpConnection::connect_with_private_key(
            &host,
            22,
            &user,
            std::path::Path::new(&key_path),
            Some(std::path::Path::new(&pub_key_path)),
        )
        .expect("実SFTPサーバーへの接続に失敗");
        let manifest = conn
            .build_remote_manifest(&remote_dir)
            .expect("マニフェスト構築に失敗");
        assert!(!manifest.is_empty());
        assert!(manifest.contains_key("sample.txt"));
        assert!(manifest.contains_key("subdir/nested.txt"));
    }
}
