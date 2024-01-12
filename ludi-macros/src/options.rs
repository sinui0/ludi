use darling::{
    util::{Flag, Override},
    FromMeta,
};
use proc_macro_error::emit_error;

#[derive(Default, Clone, FromMeta)]
pub(crate) struct MsgOptions {
    /// Attributes which are passed through to the message struct
    pub(crate) attrs: Option<NestedAttrs>,
    /// Visibility of the generated message struct
    pub(crate) vis: Option<syn::Visibility>,
    /// Message struct name
    pub(crate) name: Option<String>,
    /// Path to the message struct module
    pub(crate) path: Option<syn::Path>,
    /// Wrap options
    pub(crate) wrap: Option<Override<WrapOptions>>,
    /// Skip messages
    pub(crate) skip: Flag,
    /// Skip message handler
    pub(crate) skip_handler: Flag,
    /// Generate messages for a foreign trait
    pub(crate) foreign: Flag,
}

impl MsgOptions {
    pub(crate) fn merge(&mut self, other: &Self) {
        if let Some(attrs) = &mut self.attrs {
            if let Some(other_attrs) = &other.attrs {
                attrs.0.extend_from_slice(&other_attrs.0);
            }
        } else {
            self.attrs = other.attrs.clone();
        }

        if other.vis.is_some() {
            self.vis = other.vis.clone();
        }

        if other.name.is_some() {
            self.name = other.name.clone();
        }

        if other.path.is_some() {
            self.path = other.path.clone();
        }

        if let Some(Override::Explicit(wrap)) = &mut self.wrap {
            if let Some(Override::Explicit(other_wrap)) = other.wrap.as_ref() {
                wrap.merge(other_wrap);
            }
        } else {
            self.wrap = other.wrap.clone();
        }

        self.skip = other.skip.clone();
        self.skip_handler = other.skip_handler.clone();
        self.foreign = other.foreign.clone();
    }

    pub(crate) fn maybe_from_attributes(attrs: &[syn::Attribute]) -> Option<Self> {
        let mut any = false;
        let mut options = Self::default();
        for attr in attrs {
            if attr.path().is_ident("msg") {
                any = true;

                match &attr.meta {
                    syn::Meta::Path(_) => {
                        // We use defaults for word
                    }
                    _ => match Self::from_meta(&attr.meta) {
                        Ok(msg_options) => options.merge(&msg_options),
                        Err(err) => {
                            emit_error!(attr, "invalid `msg` attribute: {}", err);
                            return None;
                        }
                    },
                }
            }
        }

        if any {
            Some(options)
        } else {
            None
        }
    }
}

#[derive(Default, Clone, FromMeta)]
pub(crate) struct WrapOptions {
    /// Attributes which are passed through to the wrapper struct
    pub(crate) attrs: Option<NestedAttrs>,
    /// Wrapper struct ident
    pub(crate) name: Option<syn::Ident>,
}

impl WrapOptions {
    pub(crate) fn merge(&mut self, other: &Self) {
        if let Some(attrs) = &mut self.attrs {
            if let Some(other_attrs) = &other.attrs {
                attrs.0.extend_from_slice(&other_attrs.0);
            }
        } else {
            self.attrs = other.attrs.clone();
        }

        if other.name.is_some() {
            self.name = other.name.clone();
        }
    }
}

#[derive(Default, Clone, FromMeta)]
pub(crate) struct CtrlOptions {
    /// Attributes which are passed through to the controller struct
    pub(crate) attrs: Option<NestedAttrs>,
    /// Controller struct ident
    pub(crate) name: Option<syn::Ident>,
    /// Path to the controller struct
    pub(crate) path: Option<syn::Path>,
    /// Error handling
    pub(crate) err: Option<Override<syn::Expr>>,
}

impl CtrlOptions {
    pub(crate) fn merge(&mut self, other: &Self) {
        if let Some(attrs) = &mut self.attrs {
            if let Some(other_attrs) = &other.attrs {
                attrs.0.extend_from_slice(&other_attrs.0);
            }
        } else {
            self.attrs = other.attrs.clone();
        }

        if other.name.is_some() {
            self.name = other.name.clone();
        }

        if other.path.is_some() {
            self.path = other.path.clone();
        }

        if other.err.is_some() {
            self.err = other.err.clone();
        }
    }

    pub(crate) fn error_strategy(&self) -> ErrorStrategy {
        if let Some(err) = &self.err {
            match err {
                Override::Inherit => ErrorStrategy::Try,
                Override::Explicit(expr) => ErrorStrategy::Map(expr.clone()),
            }
        } else {
            ErrorStrategy::Panic
        }
    }

    pub(crate) fn maybe_from_attributes(attrs: &[syn::Attribute]) -> Option<Self> {
        let mut any = false;
        let mut options = Self::default();
        for attr in attrs {
            if attr.path().is_ident("ctrl") {
                any = true;
                match &attr.meta {
                    syn::Meta::Path(_) => {
                        // We use defaults for word
                    }
                    _ => match Self::from_meta(&attr.meta) {
                        Ok(msg_options) => options.merge(&msg_options),
                        Err(err) => {
                            emit_error!(attr, "invalid `msg` attribute: {}", err);
                            return None;
                        }
                    },
                }
            }
        }

        if any {
            Some(options)
        } else {
            None
        }
    }
}

pub(crate) enum ErrorStrategy {
    /// Panic on error
    Panic,
    /// Attempts to handle error with `?` operator
    Try,
    /// Map error to another type then attempts to handle it with `?` operator
    Map(syn::Expr),
}

impl Default for ErrorStrategy {
    fn default() -> Self {
        Self::Panic
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct NestedAttrs(Vec<darling::ast::NestedMeta>);

impl NestedAttrs {
    pub(crate) fn to_vec(self) -> Vec<darling::ast::NestedMeta> {
        self.0
    }
}

impl FromMeta for NestedAttrs {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        Ok(Self(items.to_vec()))
    }
}

impl AsRef<[darling::ast::NestedMeta]> for NestedAttrs {
    fn as_ref(&self) -> &[darling::ast::NestedMeta] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_msg_from_attributes_none() {
        let options = MsgOptions::maybe_from_attributes(&[]);
        assert!(options.is_none());
    }

    #[test]
    fn test_msg_from_attribute() {
        let attrs = vec![
            parse_quote!(#[msg(name = "Foo")]),
            parse_quote!(#[other_attr(foo = "bar")]),
        ];

        let options = MsgOptions::maybe_from_attributes(&attrs).unwrap();

        assert_eq!(options.name.unwrap().to_string(), "Foo");
    }

    #[test]
    fn test_msg_from_attributes_many() {
        let attrs = vec![
            parse_quote!(#[msg(name = "Foo")]),
            parse_quote!(#[msg(name = "Bar")]),
            parse_quote!(#[msg(vis = pub(crate))]),
        ];

        let options = MsgOptions::maybe_from_attributes(&attrs).unwrap();

        assert!(options.attrs.is_none());
        assert_eq!(options.name.unwrap().to_string(), "Bar");
        assert!(matches!(
            options.vis.unwrap(),
            syn::Visibility::Restricted(_)
        ));
    }
}
