//! Where a service is reached: a bus address or a socket, or nowhere.

/// An address that may not have been given.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Endpoint {
    /// Nothing was given: the service is not offered, or nothing is sent.
    #[default]
    Unset,
    /// The address: a bus address, a socket path, or `@name` for an abstract socket.
    At(String),
}

impl Endpoint {
    /// The endpoint for an environment variable's value (empty is none).
    ///
    /// ```
    /// use casement::endpoint::Endpoint;
    /// assert_eq!(Endpoint::of(None), Endpoint::Unset);
    /// assert_eq!(Endpoint::of(Some("")), Endpoint::Unset);
    /// assert_eq!(Endpoint::of(Some("/run/n")), Endpoint::At("/run/n".to_owned()));
    /// ```
    pub fn of(value: Option<&str>) -> Endpoint {
        match value {
            Some(text) if !text.is_empty() => Endpoint::At(text.to_owned()),
            _ => Endpoint::Unset,
        }
    }
}
