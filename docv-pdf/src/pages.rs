use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    objects::Objects,
    structures::root::pages::{InheritableAttributes, Page, PagesTreeNode},
    types::IndirectReference,
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Pages<'a> {
    root: PagesTreeNode,
    stack: Vec<(std::vec::IntoIter<IndirectReference>, InheritableAttributes)>,
    current_iter: std::vec::IntoIter<IndirectReference>,
    current_inheritable: InheritableAttributes,
    objects: &'a mut Objects,
}

impl<'a> std::iter::Iterator for Pages<'a> {
    type Item = std::result::Result<Page, crate::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.compute_next().context(crate::error::Pages) {
            Ok(Some(val)) => Some(Ok(val)),
            Ok(None) => None,
            Err(err) => Some(Err(err.into())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, Some(self.root.leaf_count))
    }
}

impl<'a> Pages<'a> {
    pub fn new(pages: &PagesTreeNode, objects: &'a mut Objects) -> Self {
        Self {
            root: pages.clone(),
            stack: Vec::new(),
            current_iter: pages.kids.clone().into_iter(),
            current_inheritable: pages.inheritable_attributes.clone(),
            objects,
        }
    }

    fn compute_next(&mut self) -> Result<Option<Page>> {
        loop {
            if let Some(kid_ref) = self.current_iter.next() {
                let kid_obj = self
                    .objects
                    .get_object(&kid_ref)
                    .context(error::ObjectNotFound {
                        reference: kid_ref,
                        field: "Pages",
                    })?;
                let dictionary = kid_obj.as_dictionary().context(error::InvalidKidType {
                    field: "Kids",
                    indirect_reference: kid_ref,
                })?;
                let node_type = dictionary
                    .get("Type")
                    .and_then(|obj| obj.as_name().ok())
                    .context(error::FieldNotFound { field: "Type" })?;

                match node_type {
                    "Page" => {
                        return Ok(Some(
                            Page::from_dictionary(
                                dictionary,
                                &self.current_inheritable,
                                self.objects,
                            )
                            .context(error::InvalidPage)?,
                        ));
                    }
                    "Pages" => {
                        let new_node = PagesTreeNode::from_dictionary(
                            dictionary,
                            Some(self.current_inheritable.clone()),
                        )
                        .context(error::InvalidPageNode)?;
                        let old_iter =
                            std::mem::replace(&mut self.current_iter, new_node.kids.into_iter());
                        let old_inheritable = std::mem::replace(
                            &mut self.current_inheritable,
                            new_node.inheritable_attributes,
                        );

                        self.stack.push((old_iter, old_inheritable));
                    }
                    _ => {
                        return Err(error::Error::UnexpectedNodeType {
                            got: node_type.to_string(),
                        }
                        .into());
                    }
                }
            } else if let Some((parent_iter, parent_inheritable)) = self.stack.pop() {
                self.current_iter = parent_iter;
                self.current_inheritable = parent_inheritable;
            } else {
                return Ok(None);
            }
        }
    }
}

mod error {
    use snafu::Snafu;

    use crate::types::IndirectReference;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Required field `{field}` not found"))]
        FieldNotFound { field: &'static str },

        #[snafu(display(
            "Invalid object type for field `{field}`. Used ref: `{indirect_reference}`"
        ))]
        InvalidKidType {
            field: &'static str,
            indirect_reference: IndirectReference,
            source: crate::types::object::Error,
        },

        #[snafu(display("Object with reference `{reference}` for field `{field}` not found"))]
        ObjectNotFound {
            reference: IndirectReference,
            field: &'static str,
            source: crate::objects::Error,
        },

        #[snafu(display("Unexpected node type. Got = `{got}`. Expected `Page` or `Pages`]"))]
        UnexpectedNodeType { got: String },

        #[snafu(display("Failed to read page data"))]
        InvalidPage {
            source: crate::structures::root::pages::Error,
        },

        #[snafu(display("Failed to read page tree node data"))]
        InvalidPageNode {
            source: crate::structures::root::pages::Error,
        },
    }
}
