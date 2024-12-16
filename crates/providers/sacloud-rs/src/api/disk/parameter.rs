use crate::enums::EDiskConnection;
use serde::Serialize;

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
struct Plan {
    i_d: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SourceArchive {
    i_d: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SSHKey {
    public_key: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    s_s_h_key: Option<SSHKey>,
}

impl Config {
    pub fn password<S: ToString>(mut self, s: S) -> Self {
        self.password.replace(s.to_string());
        self
    }

    pub fn ssh_key<S: ToString>(mut self, s: S) -> Self {
        let ssh_key = SSHKey {
            public_key: s.to_string(),
        };
        self.s_s_h_key.replace(ssh_key);
        self
    }
}

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Disk {
    name: String,
    plan: Plan,
    #[serde(skip_serializing_if = "Option::is_none")]
    connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // TOCONFIRM: without this optional field as described from the api doc,
    // creating disk will fails.
    size_m_b: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_archive: Option<SourceArchive>,
}

impl Disk {
    pub fn name<S: ToString>(mut self, s: S) -> Self {
        self.name = s.to_string();
        self
    }

    pub fn plan(mut self, id: usize) -> Self {
        self.plan = Plan { i_d: id };
        self
    }

    pub fn connection(mut self, connection: EDiskConnection) -> Self {
        self.connection.replace(connection.as_str().to_string());
        self
    }

    pub fn size_m_b(mut self, size_mb: usize) -> Self {
        self.size_m_b.replace(size_mb);
        self
    }

    pub fn source_archive_id(mut self, id: usize) -> Self {
        self.source_archive.replace(SourceArchive { i_d: id });
        self
    }
}

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Params {
    disk: Disk,
}

impl Params {
    pub fn disk(mut self, disk: Disk) -> Self {
        self.disk = disk;
        self
    }
}
