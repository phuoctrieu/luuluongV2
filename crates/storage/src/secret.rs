//! API key trong kho khóa của hệ điều hành (Keychain / Credential Manager).

use thiserror::Error;

#[derive(Debug, Error)]
#[error("Không lưu được API key vào kho khóa của hệ điều hành.")]
pub struct SecretError;

pub trait SecretStore {
    fn get(&self) -> Option<String>;
    fn set(&self, value: &str) -> Result<(), SecretError>;
    fn delete(&self) -> Result<(), SecretError>;
}

pub struct KeyringStore {
    entry: keyring::Entry,
}

impl KeyringStore {
    pub fn new(service: &str, user: &str) -> Result<Self, SecretError> {
        keyring::Entry::new(service, user).map(|entry| Self { entry }).map_err(|_| SecretError)
    }
}

impl SecretStore for KeyringStore {
    fn get(&self) -> Option<String> {
        self.entry.get_password().ok()
    }

    fn set(&self, value: &str) -> Result<(), SecretError> {
        self.entry.set_password(value).map_err(|_| SecretError)
    }

    fn delete(&self) -> Result<(), SecretError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(SecretError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> KeyringStore {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        KeyringStore::new("luuluong-test", "api-key").unwrap()
    }

    #[test]
    fn empty_until_set() {
        assert_eq!(store().get(), None);
    }

    #[test]
    fn set_then_get() {
        let s = store();
        s.set("abc123").unwrap();
        assert_eq!(s.get(), Some("abc123".into()));
    }

    #[test]
    fn delete_clears_the_key_and_is_idempotent() {
        let s = store();
        s.set("abc123").unwrap();
        s.delete().unwrap();
        assert_eq!(s.get(), None);
        s.delete().unwrap();
    }
}
