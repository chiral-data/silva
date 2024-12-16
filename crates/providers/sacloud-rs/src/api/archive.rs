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
