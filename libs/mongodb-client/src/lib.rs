mod error;

pub use error::*;

use std::str::FromStr;

use mongodb::{
    Client,
    options::{ClientOptions, DriverInfo, ResolverConfig},
};

/// A wrapper to create a new MongoDB client. Please remove me when we do not
/// need special setup anymore for this.
pub async fn create(connection_string: impl AsRef<str>) -> Result<Client, Error> {
    let mut connection_string_parser = ClientOptions::parse(connection_string.as_ref());
    if cfg!(target_os = "windows") {
        connection_string_parser = connection_string_parser.resolver_config(ResolverConfig::cloudflare());
    }

    let mut options = connection_string_parser.await?;
    options.driver_info = Some(DriverInfo::builder().name("Prisma").build());

    Ok(Client::with_options(options)?)
}

/// The parts we need taken from `mongodb` private functions. Please remove everything after me
/// when they make these apis public.
pub struct MongoConnectionString {
    pub user: Option<String>,
    pub hosts: Vec<(String, Option<u16>)>,
    pub database: String,
}

impl MongoConnectionString {
    pub fn host_strings(&self) -> Vec<String> {
        self.hosts
            .iter()
            .map(|(h, p)| match p {
                Some(p) => format!("{h}:{p}"),
                None => h.to_owned(),
            })
            .collect::<Vec<_>>()
    }
}

/// :( :( :(
impl FromStr for MongoConnectionString {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let end_of_scheme = match s.find("://") {
            Some(index) => index,
            None => {
                return Err(ErrorKind::invalid_argument("connection string contains no scheme").into());
            }
        };

        let srv = match &s[..end_of_scheme] {
            "mongodb" => false,
            "mongodb+srv" => true,
            _ => {
                return Err(ErrorKind::invalid_argument(format!(
                    "invalid connection string scheme: {}",
                    &s[..end_of_scheme]
                ))
                .into());
            }
        };

        let after_scheme = &s[end_of_scheme + 3..];

        let (pre_slash, post_slash) = match after_scheme.find('/') {
            Some(slash_index) => match exclusive_split_at(after_scheme, slash_index) {
                (Some(section), o) => (section, o),
                (None, _) => {
                    return Err(ErrorKind::invalid_argument("missing hosts").into());
                }
            },
            None => {
                if after_scheme.find('?').is_some() {
                    return Err(
                        ErrorKind::invalid_argument("Missing delimiting slash between hosts and options").into(),
                    );
                }

                (after_scheme, None)
            }
        };

        let database = match post_slash {
            Some(section) => match section.find('?') {
                Some(index) => exclusive_split_at(section, index).0,
                None => post_slash,
            },
            None => None,
        };

        let database = match database {
            Some(db) => {
                let decoded = percent_decode(db, "database name must be URL encoded")?;

                if decoded.chars().any(|c| ['/', '\\', ' ', '"', '$', '.'].contains(&c)) {
                    return Err(ErrorKind::invalid_argument("illegal character in database name").into());
                }

                decoded
            }
            None => {
                return Err(ErrorKind::invalid_argument("Database must be defined in the connection string").into());
            }
        };

        let (cred_section, hosts_section) = match pre_slash.rfind('@') {
            Some(index) => {
                // if '@' is in the host section, it MUST be interpreted as a request for
                // authentication, even if the credentials are empty.
                let (creds, hosts) = exclusive_split_at(pre_slash, index);

                match hosts {
                    Some(hs) => (creds, hs),
                    None => {
                        return Err(ErrorKind::invalid_argument("missing hosts").into());
                    }
                }
            }
            None => (None, pre_slash),
        };

        let user = match cred_section {
            Some(creds) => match creds.find(':') {
                Some(index) => exclusive_split_at(creds, index).0.map(ToString::to_string),
                None => Some(creds.to_string()), // Lack of ":" implies whole string is username
            },
            None => None,
        };

        let hosts: Result<Vec<_>, Error> = hosts_section
            .split(',')
            .map(|address| {
                let mut parts = address.split(':');

                let hostname = match parts.next() {
                    Some(part) => {
                        if part.is_empty() {
                            return Err(ErrorKind::invalid_argument(format!(
                                "invalid server address: \"{address}\"; hostname cannot be empty"
                            ))
                            .into());
                        }
                        part
                    }
                    None => {
                        return Err(
                            ErrorKind::invalid_argument(format!("invalid server address: \"{address}\"")).into(),
                        );
                    }
                };

                let port = match parts.next() {
                    Some(part) => {
                        let port = u16::from_str(part).map_err(|_| {
                            ErrorKind::invalid_argument(format!(
                                "port must be valid 16-bit unsigned integer, instead got: {part}"
                            ))
                        })?;

                        if port == 0 {
                            return Err(ErrorKind::invalid_argument(format!(
                                "invalid server address: \"{address}\"; port must be non-zero"
                            ))
                            .into());
                        }
                        if parts.next().is_some() {
                            return Err(ErrorKind::invalid_argument(format!(
                                "address \"{address}\" contains more than one unescaped ':'"
                            ))
                            .into());
                        }

                        Some(port)
                    }
                    None => None,
                };

                Ok((hostname.to_lowercase(), port))
            })
            .collect();

        let hosts = hosts?;

        if srv {
            if hosts.len() != 1 {
                return Err(
                    ErrorKind::invalid_argument("exactly one host must be specified with 'mongodb+srv'").into(),
                );
            }

            if hosts[0].1.is_some() {
                return Err(ErrorKind::invalid_argument("a port cannot be specified with 'mongodb+srv'").into());
            }
        }

        Ok(Self { user, hosts, database })
    }
}

/// Splits a string into a section before a given index and a section exclusively after the index.
/// Empty portions are returned as `None`.
fn exclusive_split_at(s: &str, i: usize) -> (Option<&str>, Option<&str>) {
    let (l, r) = s.split_at(i);

    let lout = if !l.is_empty() { Some(l) } else { None };
    let rout = if r.len() > 1 { Some(&r[1..]) } else { None };

    (lout, rout)
}

fn percent_decode(s: &str, err_message: &str) -> Result<String, Error> {
    match percent_encoding::percent_decode_str(s).decode_utf8() {
        Ok(result) => Ok(result.to_string()),
        Err(_) => Err(ErrorKind::invalid_argument(err_message).into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::MongoConnectionString;

    #[test]
    fn only_host() {
        let MongoConnectionString {
            user,
            hosts,
            database: _,
        } = "mongodb://localhost/test".parse().unwrap();

        assert_eq!(None, user.as_deref());
        assert_eq!(vec![(String::from("localhost"), None)], hosts);
    }

    #[test]
    fn srv_host() {
        let MongoConnectionString {
            user,
            hosts,
            database: _,
        } = "mongodb+srv://localhost/test".parse().unwrap();

        assert_eq!(None, user.as_deref());
        assert_eq!(vec![(String::from("localhost"), None)], hosts);
    }

    #[test]
    fn host_and_port() {
        let MongoConnectionString {
            user,
            hosts,
            database: _,
        } = "mongodb://localhost:1234/test".parse().unwrap();

        assert_eq!(None, user.as_deref());
        assert_eq!(vec![(String::from("localhost"), Some(1234))], hosts);
    }

    #[test]
    fn username() {
        let MongoConnectionString {
            user,
            hosts,
            database: _,
        } = "mongodb://username:password@localhost/test".parse().unwrap();

        assert_eq!(Some("username"), user.as_deref());
        assert_eq!(vec![(String::from("localhost"), None)], hosts);
    }

    #[test]
    fn database() {
        let MongoConnectionString { user, hosts, database } = "mongodb://localhost/foo".parse().unwrap();

        assert_eq!(None, user);
        assert_eq!("foo", database);
        assert_eq!(vec![(String::from("localhost"), None)], hosts);
    }

    #[test]
    fn sharded() {
        let s = "mongodb://prisma:risima@srv1.bu2lt.mongodb.net:27017,srv2.bu2lt.mongodb.net:27017,srv3.bu2lt.mongodb.net:27017/test?retryWrites=true&w=majority";

        let MongoConnectionString { user, hosts, database } = s.parse().unwrap();

        assert_eq!(Some("prisma"), user.as_deref());
        assert_eq!("test", database);

        assert_eq!(
            vec![
                (String::from("srv1.bu2lt.mongodb.net"), Some(27017)),
                (String::from("srv2.bu2lt.mongodb.net"), Some(27017)),
                (String::from("srv3.bu2lt.mongodb.net"), Some(27017)),
            ],
            hosts
        );
    }
}
