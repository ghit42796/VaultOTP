use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl Default for Algorithm {
    fn default() -> Self { Algorithm::Sha1 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub issuer: String,
    pub label: String,
    pub secret: String,
    #[serde(default)]
    pub algorithm: Algorithm,
    pub digits: u32,
    pub period: u64,
    #[serde(rename = "type", default = "default_type")]
    pub kind: String,
}

fn default_type() -> String { "totp".to_string() }

impl Account {
    pub fn new(issuer: String, label: String, secret: String) -> Self {
        Account {
            id: String::new(),
            issuer,
            label,
            secret,
            algorithm: Algorithm::Sha1,
            digits: 6,
            period: 30,
            kind: "totp".to_string(),
        }
    }
}

impl Drop for Account {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_account_has_empty_id_and_defaults() {
        let a = Account::new("GitHub".into(), "alice".into(), "JBSWY3DPEHPK3PXP".into());
        assert_eq!(a.id, "");
        assert_eq!(a.digits, 6);
        assert_eq!(a.period, 30);
        assert_eq!(a.algorithm, Algorithm::Sha1);
        assert_eq!(a.kind, "totp");
    }
}
