use std::borrow::{Borrow, Cow};
use std::convert::TryFrom;
use std::fmt::{self, Display};
use std::ops::{Deref, Div};
use std::path::{Component, Path, PathBuf};

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct PublicPath<'a>(Cow<'a, Path>);

impl<'a> PublicPath<'a> {
    pub fn new() -> Self {
        Self(Cow::Borrowed("public".as_ref()))
    }

    fn try_from_path(path: Cow<'a, Path>) -> Result<Self, Error> {
        if Self::check(&path) {
            Ok(Self(path))
        } else {
            Err(Error::IllegalResource(path.to_string_lossy().to_string()))
        }
    }

    // checks that a path doesn't point to anything outside of public
    // recursing into public/ increases `level`
    // going outside (../) decreases `level`
    // as soon as we step out of public/ (`level` < 1`), return false
    // prefix (C:) and root (/) also return false
    // in every other case, return true
    fn check<P: AsRef<Path>>(path: P) -> bool {
        let mut level = 1;
        for c in path.as_ref().components() {
            match c {
                Component::Prefix(_) => return false,
                Component::RootDir => return false,
                Component::CurDir => {}
                Component::ParentDir => {
                    level -= 1;
                    if level < 1 {
                        return false;
                    }
                }
                Component::Normal(_) => level += 1,
            }
        }
        true
    }
}

impl<'a, 'b> TryFrom<&'b Path> for PublicPath<'a> {
    type Error = Error;

    fn try_from(path: &'b Path) -> Result<Self, Self::Error> {
        PublicPath::new() / path
    }
}

impl<'a> TryFrom<PathBuf> for PublicPath<'a> {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        PublicPath::new() / path
    }
}

impl<'a, 'b> TryFrom<&'b str> for PublicPath<'a> {
    type Error = Error;

    fn try_from(path: &'b str) -> Result<Self, Self::Error> {
        PublicPath::new() / path
    }
}

impl<'a> TryFrom<String> for PublicPath<'a> {
    type Error = Error;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        PublicPath::new() / path
    }
}

impl<'a, 'b, T: 'b> Div<T> for PublicPath<'a>
where
    T: AsRef<Path>,
{
    type Output = Result<Self, Error>;

    fn div(self, other: T) -> Self::Output {
        Self::try_from_path(Cow::Owned(self.0.join(other.as_ref())))
    }
}

impl<'a> Display for PublicPath<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.0.as_ref().display(), f)
    }
}

impl<'a> AsRef<Path> for PublicPath<'a> {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl<'a> Borrow<Path> for PublicPath<'a> {
    fn borrow(&self) -> &Path {
        self.0.as_ref()
    }
}

impl<'a> Deref for PublicPath<'a> {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
