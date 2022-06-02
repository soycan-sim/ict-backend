use actix_http::client::SendRequestError;
use actix_web::{error::JsonPayloadError, ResponseError};
use ron::de::Error as RonError;
use serde_json::Error as JsonError;
use std::fmt::{self, Display};
use std::io::Error as IoError;
use std::num::ParseIntError;
use tokio_postgres::Error as DbError;

#[derive(Debug)]
pub enum Error {
    Ron(RonError),
    Json(JsonError),
    JsonPayload(JsonPayloadError),
    AwcJsonPayload(awc::error::JsonPayloadError),
    SendRequest(SendRequestError),
    Db(DbError),
    Io(IoError),
    Template(ParseIntError),
    Cmdline(String),
    Useradd,
    CreateDb,
    ResourceNotFound(String),
    IllegalResource(String),
    Argon2(argon2::Error),
    AuthenticationFailed,
    AuthorizationFailed,
    PasswordMismatch,
    InvalidCreateUser(String),
    InvalidPattern(String),
    AsyncRecursion,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Ron(err) => Display::fmt(err, f),
            Error::Json(err) => Display::fmt(err, f),
            Error::JsonPayload(err) => Display::fmt(err, f),
            Error::AwcJsonPayload(err) => Display::fmt(err, f),
            Error::SendRequest(err) => Display::fmt(err, f),
            Error::Db(err) => Display::fmt(err, f),
            Error::Io(err) => Display::fmt(err, f),
            Error::Template(err) => write!(f, "template error: {}", err),
            Error::Cmdline(err) => write!(f, "command line error: {}", err),
            Error::Useradd => write!(f, "creating the user `ict` failed (`useradd ... ict`)"),
            Error::CreateDb => write!(f, "creating the database `ict` failed (`createdb ... ict`)"),
            Error::ResourceNotFound(res) => write!(f, "resource not found: {:?}", res),
            Error::IllegalResource(res) => write!(f, "illegal resources: {:?}", res),
            Error::Argon2(err) => write!(f, "an error occured while trying authenticate: {}", err),
            Error::AuthenticationFailed => write!(f, "authentication failed"),
            Error::AuthorizationFailed => write!(f, "authorization failed"),
            Error::PasswordMismatch => write!(f, "passwords didn't match"),
            Error::InvalidCreateUser(desc) => {
                write!(f, "invalid user creation parameter: {}", desc)
            }
            Error::InvalidPattern(pat) => write!(f, "invalid pattern: {:?}", pat),
            Error::AsyncRecursion => write!(f, "async recursion"),
        }
    }
}

impl From<DbError> for Error {
    fn from(err: DbError) -> Error {
        Error::Db(err)
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Error::Io(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        Error::Template(err)
    }
}

impl From<argon2::Error> for Error {
    fn from(err: argon2::Error) -> Error {
        Error::Argon2(err)
    }
}

impl From<RonError> for Error {
    fn from(err: RonError) -> Error {
        Error::Ron(err)
    }
}

impl From<JsonError> for Error {
    fn from(err: JsonError) -> Error {
        Error::Json(err)
    }
}

impl From<JsonPayloadError> for Error {
    fn from(err: JsonPayloadError) -> Error {
        Error::JsonPayload(err)
    }
}

impl From<awc::error::JsonPayloadError> for Error {
    fn from(err: awc::error::JsonPayloadError) -> Error {
        Error::AwcJsonPayload(err)
    }
}

impl From<SendRequestError> for Error {
    fn from(err: SendRequestError) -> Error {
        Error::SendRequest(err)
    }
}

impl ResponseError for Error {}

pub type Result<T> = std::result::Result<T, Error>;
