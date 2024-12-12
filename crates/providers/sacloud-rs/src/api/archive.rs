//! Sakura Internet API - Archive
//! https://manual.sakura.ad.jp/cloud-api/1.1/archive/index.html
//!
//! - [x] GET    /archive                          - アーカイブ一覧を取得
//! - [ ] POST   /archive                          - アーカイブを作成
//! - [ ] GET    /archive/:archiveid               - 該当IDのアーカイブ情報を取得
//! - [ ] PUT    /archive/:archiveid               - アーカイブを更新
//! - [ ] DELETE /archive/:archiveid               - 該当IDのアーカイブを削除
//! - [ ] PUT    /archive/:archiveid/ftp           - アーカイブのFTP共有を開始 または リセットする
//! - [ ] DELETE /archive/:archiveid/ftp           - アーカイブのFTP共有を終了し、利用可能な状態にする
//! - [ ] GET    /archive/:archiveid/tag           - 該当IDのアーカイブに付けられたタグを取得
//! - [ ] PUT    /archive/:archiveid/tag           - 該当IDのアーカイブに付けられるタグを変更
//! - [ ] POST   /archive/:archiveid/to/zone/:zoneid - アーカイブを他のゾーンに転送
//! - [ ] GET    /archive/tag                      - アーカイブタグ一覧を取得

use serde::{Deserialize, Serialize};

create_struct!(Archive, "PascalCase",
    i_d: String,
    name: String,
    description: String
);

create_struct!(ArchiveList, "PascalCase",
    from: usize,
    count: usize,
    total: usize,
    archives: Vec<Archive>
);

#[cfg(test)]
mod tests {
    use crate::Client;
    use crate::Zone;

    use super::*;

    #[tokio::test]
    async fn test_get_archive() {
        let client = Client::default().set_zone(Zone::Tokyo2);
        let al: ArchiveList = client.clear().archive().get().await.unwrap();
        assert_eq!(al.total, 73);
    }
}
