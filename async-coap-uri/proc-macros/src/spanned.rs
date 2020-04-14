use std::ops::{Deref, DerefMut};

use proc_macro2::Span;

/// Attach a [`Span`] to an arbitrary type.
#[derive(Debug, Clone, Copy, Default)]
pub struct Spanned<T> {
    inner: T,
    span: Option<Span>,
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/// Allows to compare for example a `Spanned<bool>` with a `bool`
impl<T: PartialEq> PartialEq<T> for Spanned<T> {
    fn eq(&self, other: &T) -> bool {
        &self.inner == other
    }
}

impl<T: Eq> Eq for Spanned<T> {}

impl<T: ::core::hash::Hash> ::core::hash::Hash for Spanned<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: ::core::hash::Hasher,
    {
        self.inner.hash(state)
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> ::syn::spanned::Spanned for Spanned<T> {
    fn span(&self) -> Span {
        self.span.unwrap_or_else(Span::call_site)
    }
}

impl<T> DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Spanned<T> {
    #[must_use]
    pub const fn new(inner: T) -> Self {
        Self { inner, span: None }
    }

    #[must_use]
    pub fn with_span<S: ::syn::spanned::Spanned>(mut self, span: &S) -> Self {
        if self.span.is_none() {
            self.span = Some(span.span());
        }

        self
    }
}

impl<T> From<T> for Spanned<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}
