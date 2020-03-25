use std::borrow::{Borrow, Cow};
use std::fmt::{self, Display};
use std::ops::{Deref, Div};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PublicPath<'a>(Cow<'a, Path>);

impl<'a> PublicPath<'a> {
    pub fn new() -> Self {
        Self(Cow::Borrowed("public".as_ref()))
    }
}

impl<'a, 'b> From<&'b Path> for PublicPath<'a> {
    fn from(path: &'b Path) -> Self {
        PublicPath::new() / path
    }
}

impl<'a> From<PathBuf> for PublicPath<'a> {
    fn from(path: PathBuf) -> Self {
        PublicPath::new() / path
    }
}

impl<'a, 'b> From<&'b str> for PublicPath<'a> {
    fn from(path: &'b str) -> Self {
        PublicPath::new() / path
    }
}

impl<'a> From<String> for PublicPath<'a> {
    fn from(path: String) -> Self {
        PublicPath::new() / path
    }
}

impl<'a, 'b, T: 'b> Div<T> for PublicPath<'a>
where
    T: AsRef<Path>,
{
    type Output = Self;

    fn div(self, other: T) -> Self {
        Self(Cow::Owned(self.0.join(other.as_ref())))
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
