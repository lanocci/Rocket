use std::fmt;

use crate::Header;

pub use cookie::{Cookie, CookieCrumb, SameSite, Iter};
#[doc(hidden)] pub use self::key::*;

/// Types and methods to manage a `Key` when private cookies are enabled.
#[cfg(feature = "private-cookies")]
mod key {
    pub use cookie::Key;
}

/// Types and methods to manage a `Key` when private cookies are disabled.
#[cfg(not(feature = "private-cookies"))]
mod key {
    #[derive(Copy, Clone)]
    pub struct Key;

    impl Key {
        pub fn generate() -> Self { Key }
        pub fn try_generate() -> Option<Self> { Some(Key) }
        pub fn derive_from(_bytes: &[u8]) -> Self { Key }
    }
}

/// Collection of one or more HTTP cookies.
///
/// The `CookieJar` type allows for retrieval of cookies from an incoming
/// request as well as modifications to cookies to be reflected by Rocket on
/// outgoing responses.
///
/// # Usage
///
/// A type of `&CookieJar` can be retrieved via its `FromRequest` implementation
/// as a request guard or via the [`Request::cookies()`] method. Individual
/// cookies can be retrieved via the [`get()`] and [`get_private()`] methods.
/// Cookies can be added or removed via the [`add()`], [`add_private()`],
/// [`remove()`], and [`remove_private()`] methods.
///
/// [`Request::cookies()`]: rocket::Request::cookies()
/// [`get()`]: #method.get
/// [`get_private()`]: #method.get_private
/// [`add()`]: #method.add
/// [`add_private()`]: #method.add_private
/// [`remove()`]: #method.remove
/// [`remove_private()`]: #method.remove_private
///
/// ## Examples
///
/// The following example shows `&CookieJar` being used as a request guard in a
/// handler to retrieve the value of a "message" cookie.
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::http::CookieJar;
///
/// #[get("/message")]
/// fn message(jar: &CookieJar<'_>) -> Option<String> {
///     jar.get("message").map(|c| format!("Message: {}", c.value()))
/// }
/// # fn main() {  }
/// ```
///
/// The following snippet shows `&CookieJar` being retrieved from a `Request` in
/// a custom request guard implementation for `User`. A [private cookie]
/// containing a user's ID is retrieved. If the cookie exists and the ID parses
/// as an integer, a `User` structure is validated. Otherwise, the guard
/// forwards.
///
/// [private cookie]: #method.add_private
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// #
/// use rocket::http::Status;
/// use rocket::outcome::IntoOutcome;
/// use rocket::request::{self, Request, FromRequest};
///
/// // In practice, we'd probably fetch the user from the database.
/// struct User(usize);
///
/// #[rocket::async_trait]
/// impl<'a, 'r> FromRequest<'a, 'r> for User {
///     type Error = std::convert::Infallible;
///
///     async fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
///         request.cookies()
///             .get_private("user_id")
///             .and_then(|c| c.value().parse().ok())
///             .map(|id| User(id))
///             .or_forward(())
///     }
/// }
/// # fn main() { }
/// ```
///
/// # Private Cookies
///
/// _Private_ cookies are just like regular cookies except that they are
/// encrypted using authenticated encryption, a form of encryption which
/// simultaneously provides confidentiality, integrity, and authenticity. This
/// means that private cookies cannot be inspected, tampered with, or
/// manufactured by clients. If you prefer, you can think of private cookies as
/// being signed and encrypted.
///
/// Private cookies can be retrieved, added, and removed from a `CookieJar`
/// collection via the [`get_private()`], [`add_private()`], and
/// [`remove_private()`] methods.
///
/// ## Encryption Key
///
/// To encrypt private cookies, Rocket uses the 256-bit key specified in the
/// `secret_key` configuration parameter. If one is not specified, Rocket will
/// automatically generate a fresh key. Note, however, that a private cookie can
/// only be decrypted with the same key with which it was encrypted. As such, it
/// is important to set a `secret_key` configuration parameter when using
/// private cookies so that cookies decrypt properly after an application
/// restart. Rocket will emit a warning if an application is run in production
/// mode without a configured `secret_key`.
///
/// Generating a string suitable for use as a `secret_key` configuration value
/// is usually done through tools like `openssl`. Using `openssl`, for instance,
/// a 256-bit base64 key can be generated with the command `openssl rand -base64
/// 32`.
#[derive(Clone)]
pub struct CookieJar<'a> {
    jar: cookie::CookieJar,
    key: &'a Key,
}

impl<'a> CookieJar<'a> {
    /// Returns a reference to the `Cookie` inside this container with the name
    /// `name`. If no such cookie exists, returns `None`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::{Cookie, CookieJar};
    ///
    /// #[get("/")]
    /// fn handler(jar: &CookieJar<'_>) {
    ///     let cookie = jar.get("name");
    /// }
    /// ```
    pub fn get(&self, name: &str) -> Option<CookieCrumb> {
        self.jar.get(name)
    }

    /// Returns a reference to the `Cookie` inside this collection with the name
    /// `name` and authenticates and decrypts the cookie's value, returning a
    /// `Cookie` with the decrypted value. If the cookie cannot be found, or the
    /// cookie fails to authenticate or decrypt, `None` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::{Cookie, CookieJar};
    ///
    /// #[get("/")]
    /// fn handler(jar: &CookieJar<'_>) {
    ///     let cookie = jar.get_private("name");
    /// }
    /// ```
    #[cfg(feature = "private-cookies")]
    #[cfg_attr(nightly, doc(cfg(feature = "secrets")))]
    pub fn get_private(&self, name: &str) -> Option<Cookie<'static>> {
        self.jar.private(&*self.key).get(name)
    }

    /// Adds `cookie` to this collection.
    ///
    /// Unless a value is set for the given property, the following defaults are
    /// set on `cookie` before being added to `self`:
    ///
    ///    * `path`: `"/"`
    ///    * `SameSite`: `Strict`
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::{Cookie, SameSite, CookieJar};
    ///
    /// #[get("/")]
    /// fn handler(jar: &CookieJar<'_>) {
    ///     jar.add(Cookie::new("first", "value"));
    ///
    ///     let cookie = Cookie::build("other", "value_two")
    ///         .path("/")
    ///         .secure(true)
    ///         .same_site(SameSite::Lax);
    ///
    ///     jar.add(cookie.finish());
    /// }
    /// ```
    pub fn add(&self, mut cookie: Cookie<'static>) {
        Self::set_defaults(&mut cookie);
        self.jar.add(cookie)
    }

    /// Adds `cookie` to the collection. The cookie's value is encrypted with
    /// authenticated encryption assuring confidentiality, integrity, and
    /// authenticity. The cookie can later be retrieved using
    /// [`get_private`](#method.get_private) and removed using
    /// [`remove_private`](#method.remove_private).
    ///
    /// Unless a value is set for the given property, the following defaults are
    /// set on `cookie` before being added to `self`:
    ///
    ///    * `path`: `"/"`
    ///    * `SameSite`: `Strict`
    ///    * `HttpOnly`: `true`
    ///    * `Expires`: 1 week from now
    ///
    /// These defaults ensure maximum usability and security. For additional
    /// security, you may wish to set the `secure` flag.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::{Cookie, CookieJar};
    ///
    /// #[get("/")]
    /// fn handler(jar: &CookieJar<'_>) {
    ///     jar.add_private(Cookie::new("name", "value"));
    /// }
    /// ```
    #[cfg(feature = "private-cookies")]
    #[cfg_attr(nightly, doc(cfg(feature = "secrets")))]
    pub fn add_private(&self, mut cookie: Cookie<'static>) {
        Self::set_private_defaults(&mut cookie);
        self.jar.private(&*self.key).add(cookie)
    }

    /// Removes `cookie` from this collection and generates a "removal" cookies
    /// to send to the client on response. For correctness, `cookie` must
    /// contain the same `path` and `domain` as the cookie that was initially
    /// set. Failure to provide the initial `path` and `domain` will result in
    /// cookies that are not properly removed. For convenience, if a path is not
    /// set on `cookie`, the `"/"` path will automatically be set.
    ///
    /// A "removal" cookie is a cookie that has the same name as the original
    /// cookie but has an empty value, a max-age of 0, and an expiration date
    /// far in the past.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::{Cookie, CookieJar};
    ///
    /// #[get("/")]
    /// fn handler(jar: &CookieJar<'_>) {
    ///     jar.remove(Cookie::named("name"));
    /// }
    /// ```
    pub fn remove(&self, mut cookie: Cookie<'static>) {
        if cookie.path().is_none() {
            cookie.set_path("/");
        }

        self.jar.remove(cookie)
    }

    /// Removes the private `cookie` from the collection.
    ///
    /// For correct removal, the passed in `cookie` must contain the same `path`
    /// and `domain` as the cookie that was initially set. If a path is not set
    /// on `cookie`, the `"/"` path will automatically be set.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::{Cookie, CookieJar};
    ///
    /// #[get("/")]
    /// fn handler(jar: &CookieJar<'_>) {
    ///     jar.remove_private(Cookie::named("name"));
    /// }
    /// ```
    #[cfg(feature = "private-cookies")]
    #[cfg_attr(nightly, doc(cfg(feature = "secrets")))]
    pub fn remove_private(&self, mut cookie: Cookie<'static>) {
        if cookie.path().is_none() {
            cookie.set_path("/");
        }

        self.jar.private(&*self.key).remove(cookie)
    }

    /// Returns an iterator over all of the cookies present in this collection.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::http::{Cookie, CookieJar};
    ///
    /// #[get("/")]
    /// fn handler(jar: &CookieJar<'_>) {
    ///     for c in jar.iter() {
    ///         println!("Name: {:?}, Value: {:?}", c.name(), c.value());
    ///     }
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = CookieCrumb> + '_ {
        self.jar.iter()
    }
}

/// WARNING: These is unstable! Do not use outside of Rocket!
#[doc(hidden)]
impl<'a> CookieJar<'a> {
    #[inline(always)]
    pub fn new(key: &'a Key) -> CookieJar<'a> {
        CookieJar { jar: cookie::CookieJar::new(), key }
    }

    #[inline(always)]
    pub fn from(jar: cookie::CookieJar, key: &'a Key) -> CookieJar<'a> {
        CookieJar { jar, key }
    }

    /// Removes all delta cookies.
    #[inline(always)]
    pub fn reset_delta(&self) {
        self.jar.reset_delta()
    }

    #[inline(always)]
    pub fn delta(&self) -> cookie::Delta {
        self.jar.delta()
    }

    /// Adds an original `cookie` to this collection.
    #[inline(always)]
    pub fn add_original(&self, cookie: Cookie<'static>) {
        self.jar.add_original(cookie)
    }

    /// Adds an original, private `cookie` to the collection.
    #[inline(always)]
    #[cfg(feature = "private-cookies")]
    #[cfg_attr(nightly, doc(cfg(feature = "secrets")))]
    pub fn add_original_private(&self, cookie: Cookie<'static>) {
        self.jar.private(&*self.key).add_original(cookie);
    }

    /// For each property mentioned below, this method checks if there is a
    /// provided value and if there is none, sets a default value. Default
    /// values are:
    ///
    ///    * `path`: `"/"`
    ///    * `SameSite`: `Strict`
    ///
    fn set_defaults(cookie: &mut Cookie<'static>) {
        if cookie.path().is_none() {
            cookie.set_path("/");
        }

        if cookie.same_site().is_none() {
            cookie.set_same_site(SameSite::Strict);
        }
    }

    /// For each property mentioned below, this method checks if there is a
    /// provided value and if there is none, sets a default value. Default
    /// values are:
    ///
    ///    * `path`: `"/"`
    ///    * `SameSite`: `Strict`
    ///    * `HttpOnly`: `true`
    ///    * `Expires`: 1 week from now
    ///
    #[cfg(feature = "private-cookies")]
    #[cfg_attr(nightly, doc(cfg(feature = "secrets")))]
    fn set_private_defaults(cookie: &mut Cookie<'static>) {
        if cookie.path().is_none() {
            cookie.set_path("/");
        }

        if cookie.same_site().is_none() {
            cookie.set_same_site(SameSite::Strict);
        }

        if cookie.http_only().is_none() {
            cookie.set_http_only(true);
        }

        if cookie.expires().is_none() {
            cookie.set_expires(time::OffsetDateTime::now_utc() + time::Duration::weeks(1));
        }
    }
}

impl fmt::Debug for CookieJar<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.jar.fmt(f)
    }
}

impl From<Cookie<'_>> for Header<'static> {
    fn from(cookie: Cookie<'_>) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}

impl From<&Cookie<'_>> for Header<'static> {
    fn from(cookie: &Cookie<'_>) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}

impl From<CookieCrumb> for Header<'static> {
    fn from(cookie: CookieCrumb) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}

impl From<&CookieCrumb> for Header<'static> {
    fn from(cookie: &CookieCrumb) -> Header<'static> {
        Header::new("Set-Cookie", cookie.encoded().to_string())
    }
}
