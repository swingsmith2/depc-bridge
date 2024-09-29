use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    InvalidSource,
}

pub struct Auth {
    pub user: String,
    pub passwd: String,
}

impl Auth {
    fn new() -> Auth {
        Auth {
            user: "".to_owned(),
            passwd: "".to_owned(),
        }
    }
}

struct AuthBuilder {
    auth: Auth,
}

impl AuthBuilder {
    pub fn new() -> AuthBuilder {
        AuthBuilder { auth: Auth::new() }
    }

    pub fn load_string(mut self, s: &str) -> AuthBuilder {
        if let Ok(auth) = s.try_into() {
            self.auth = auth;
        }
        self
    }

    pub fn load_default_cookie(mut self, path: &Path) -> AuthBuilder {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(auth) = content.try_into() {
                self.auth = auth;
            }
        }
        self
    }
}

impl TryFrom<&str> for Auth {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Some(pos) = value.find(":") {
            return Ok(Auth {
                user: value[0..pos].to_owned(),
                passwd: value[pos + 1..].to_owned(),
            });
        }
        Err(Error::InvalidSource)
    }
}

impl TryFrom<String> for Auth {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        TryInto::<Auth>::try_into(value.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correct_source() {
        let auth: Auth = "hello:world".try_into().unwrap();
        assert_eq!(auth.user, "hello");
        assert_eq!(auth.passwd, "world");
    }

    #[test]
    fn test_wrong_source() {
        assert!(TryInto::<Auth>::try_into("hello_world").is_err());
    }

    #[test]
    fn test_wrong_source_empty() {
        assert!(TryInto::<Auth>::try_into("").is_err());
    }

    #[test]
    fn test_empty_source() {
        let auth: Auth = ":".try_into().unwrap();
        assert_eq!(auth.user, "");
        assert_eq!(auth.passwd, "");
    }

    #[test]
    fn test_empty_user() {
        let auth: Auth = ":world".try_into().unwrap();
        assert_eq!(auth.user, "");
        assert_eq!(auth.passwd, "world");
    }

    #[test]
    fn test_empty_password() {
        let auth: Auth = "hello:".try_into().unwrap();
        assert_eq!(auth.user, "hello");
        assert_eq!(auth.passwd, "");
    }
}
